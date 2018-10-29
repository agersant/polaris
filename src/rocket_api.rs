use rocket::http::{Cookie, Cookies, RawStr, Status};
use rocket::request::{self, FromParam, FromRequest, Request};
use rocket::response::content::Html;
use rocket::{Outcome, State};
use rocket_contrib::json::Json;
use std::fs::File;
use std::ops::Deref;
use std::path::PathBuf;
use std::str;
use std::str::FromStr;
use std::sync::Arc;

use config::{self, Config, Preferences};
use db::DB;
use errors;
use index;
use lastfm;
use playlist;
use serve;
use thumbnails;
use user;
use utils;
use vfs::VFSSource;

const CURRENT_MAJOR_VERSION: i32 = 3;
const CURRENT_MINOR_VERSION: i32 = 0;
const SESSION_FIELD_USERNAME: &str = "username";

pub fn get_routes() -> Vec<rocket::Route> {
	routes![
		version,
		initial_setup,
		get_settings,
		put_settings,
		get_preferences,
		put_preferences,
		trigger_index,
		auth,
		browse_root,
		browse,
		flatten_root,
		flatten,
		random,
		recent,
		search_root,
		search,
		serve,
		list_playlists,
		save_playlist,
		read_playlist,
		delete_playlist,
		lastfm_link,
		lastfm_unlink,
		lastfm_now_playing,
		lastfm_scrobble,
	]
}

struct Auth {
	username: String,
}

fn get_auth_cookie(username: &str) -> Cookie<'static> {
	Cookie::build(SESSION_FIELD_USERNAME, username.to_owned())
		.same_site(rocket::http::SameSite::Lax)
		.finish()
}

impl<'a, 'r> FromRequest<'a, 'r> for Auth {
	type Error = ();

	fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, ()> {
		let mut cookies = request.guard::<Cookies>().unwrap();
		if let Some(u) = cookies.get_private(SESSION_FIELD_USERNAME) {
			return Outcome::Success(Auth {
				username: u.value().to_string(),
			});
		}

		if let Some(auth_header_string) = request.headers().get_one("Authorization") {
			use rocket::http::hyper::header::*;
			if let Ok(Basic {
				username,
				password: Some(password),
			}) = Basic::from_str(auth_header_string.trim_start_matches("Basic "))
			{
				let db = match request.guard::<State<Arc<DB>>>() {
					Outcome::Success(d) => d,
					_ => return Outcome::Failure((Status::InternalServerError, ())),
				};
				if user::auth(db.deref().deref(), &username, &password).unwrap_or(false) {
					cookies.add_private(get_auth_cookie(&username));
					return Outcome::Success(Auth {
						username: username.to_string(),
					});
				}
			}
		}

		Outcome::Failure((Status::Unauthorized, ()))
	}
}

struct AdminRights {}
impl<'a, 'r> FromRequest<'a, 'r> for AdminRights {
	type Error = ();

	fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, ()> {
		let db = request.guard::<State<Arc<DB>>>()?;

		match user::count::<DB>(&db) {
			Err(_) => return Outcome::Failure((Status::InternalServerError, ())),
			Ok(0) => return Outcome::Success(AdminRights {}),
			_ => (),
		};

		let auth = request.guard::<Auth>()?;
		match user::is_admin::<DB>(&db, &auth.username) {
			Err(_) => Outcome::Failure((Status::InternalServerError, ())),
			Ok(true) => Outcome::Success(AdminRights {}),
			Ok(false) => Outcome::Failure((Status::Forbidden, ())),
		}
	}
}

struct VFSPathBuf {
	path_buf: PathBuf,
}

impl<'r> FromParam<'r> for VFSPathBuf {
	type Error = &'r RawStr;

	fn from_param(param: &'r RawStr) -> Result<Self, Self::Error> {
		let decoded_path = param.percent_decode_lossy();
		Ok(VFSPathBuf {
			path_buf: PathBuf::from(decoded_path.into_owned()),
		})
	}
}

impl From<VFSPathBuf> for PathBuf {
	fn from(vfs_path_buf: VFSPathBuf) -> Self {
		vfs_path_buf.path_buf.clone()
	}
}

#[derive(Serialize)]
struct Version {
	major: i32,
	minor: i32,
}

#[get("/version")]
fn version() -> Json<Version> {
	let current_version = Version {
		major: CURRENT_MAJOR_VERSION,
		minor: CURRENT_MINOR_VERSION,
	};
	Json(current_version)
}

#[derive(Serialize)]
struct InitialSetup {
	has_any_users: bool,
}

#[get("/initial_setup")]
fn initial_setup(db: State<Arc<DB>>) -> Result<Json<InitialSetup>, errors::Error> {
	let initial_setup = InitialSetup {
		has_any_users: user::count::<DB>(&db)? > 0,
	};
	Ok(Json(initial_setup))
}

#[get("/settings")]
fn get_settings(
	db: State<Arc<DB>>,
	_admin_rights: AdminRights,
) -> Result<Json<Config>, errors::Error> {
	let config = config::read::<DB>(&db)?;
	Ok(Json(config))
}

#[put("/settings", data = "<config>")]
fn put_settings(
	db: State<Arc<DB>>,
	_admin_rights: AdminRights,
	config: Json<Config>,
) -> Result<(), errors::Error> {
	config::amend::<DB>(&db, &config)?;
	Ok(())
}

#[get("/preferences")]
fn get_preferences(db: State<Arc<DB>>, auth: Auth) -> Result<Json<Preferences>, errors::Error> {
	let preferences = config::read_preferences::<DB>(&db, &auth.username)?;
	Ok(Json(preferences))
}

#[put("/preferences", data = "<preferences>")]
fn put_preferences(
	db: State<Arc<DB>>,
	auth: Auth,
	preferences: Json<Preferences>,
) -> Result<(), errors::Error> {
	config::write_preferences::<DB>(&db, &auth.username, &preferences)?;
	Ok(())
}

#[post("/trigger_index")]
fn trigger_index(
	command_sender: State<Arc<index::CommandSender>>,
	_admin_rights: AdminRights,
) -> Result<(), errors::Error> {
	command_sender.trigger_reindex()?;
	Ok(())
}

#[derive(Deserialize)]
struct AuthCredentials {
	username: String,
	password: String,
}

#[derive(Serialize)]
struct AuthOutput {
	admin: bool,
}

#[post("/auth", data = "<credentials>")]
fn auth(
	db: State<Arc<DB>>,
	credentials: Json<AuthCredentials>,
	mut cookies: Cookies,
) -> Result<Json<AuthOutput>, errors::Error> {
	user::auth::<DB>(&db, &credentials.username, &credentials.password)?;
	cookies.add_private(get_auth_cookie(&credentials.username));

	let auth_output = AuthOutput {
		admin: user::is_admin::<DB>(&db, &credentials.username)?,
	};
	Ok(Json(auth_output))
}

#[get("/browse")]
fn browse_root(
	db: State<Arc<DB>>,
	_auth: Auth,
) -> Result<Json<Vec<index::CollectionFile>>, errors::Error> {
	let result = index::browse(db.deref().deref(), &PathBuf::new())?;
	Ok(Json(result))
}

#[get("/browse/<path>")]
fn browse(
	db: State<Arc<DB>>,
	_auth: Auth,
	path: VFSPathBuf,
) -> Result<Json<Vec<index::CollectionFile>>, errors::Error> {
	let result = index::browse(db.deref().deref(), &path.into() as &PathBuf)?;
	Ok(Json(result))
}

#[get("/flatten")]
fn flatten_root(db: State<Arc<DB>>, _auth: Auth) -> Result<Json<Vec<index::Song>>, errors::Error> {
	let result = index::flatten(db.deref().deref(), &PathBuf::new())?;
	Ok(Json(result))
}

#[get("/flatten/<path>")]
fn flatten(
	db: State<Arc<DB>>,
	_auth: Auth,
	path: VFSPathBuf,
) -> Result<Json<Vec<index::Song>>, errors::Error> {
	let result = index::flatten(db.deref().deref(), &path.into() as &PathBuf)?;
	Ok(Json(result))
}

#[get("/random")]
fn random(db: State<Arc<DB>>, _auth: Auth) -> Result<Json<Vec<index::Directory>>, errors::Error> {
	let result = index::get_random_albums(db.deref().deref(), 20)?;
	Ok(Json(result))
}

#[get("/recent")]
fn recent(db: State<Arc<DB>>, _auth: Auth) -> Result<Json<Vec<index::Directory>>, errors::Error> {
	let result = index::get_recent_albums(db.deref().deref(), 20)?;
	Ok(Json(result))
}

#[get("/search")]
fn search_root(
	db: State<Arc<DB>>,
	_auth: Auth,
) -> Result<Json<Vec<index::CollectionFile>>, errors::Error> {
	let result = index::search(db.deref().deref(), "")?;
	Ok(Json(result))
}

#[get("/search/<query>")]
fn search(
	db: State<Arc<DB>>,
	_auth: Auth,
	query: String,
) -> Result<Json<Vec<index::CollectionFile>>, errors::Error> {
	let result = index::search(db.deref().deref(), &query)?;
	Ok(Json(result))
}

#[get("/serve/<path>")]
fn serve(
	db: State<Arc<DB>>,
	_auth: Auth,
	path: VFSPathBuf,
) -> Result<serve::RangeResponder<File>, errors::Error> {
	let db: &DB = db.deref().deref();
	let vfs = db.get_vfs()?;
	let real_path = vfs.virtual_to_real(&path.into() as &PathBuf)?;

	let serve_path = if utils::is_image(&real_path) {
		thumbnails::get_thumbnail(&real_path, 400)?
	} else {
		real_path
	};

	let file = File::open(serve_path)?;
	Ok(serve::RangeResponder::new(file))
}

#[derive(Serialize)]
struct ListPlaylistsEntry {
	name: String,
}

#[get("/playlists")]
fn list_playlists(
	db: State<Arc<DB>>,
	auth: Auth,
) -> Result<Json<Vec<ListPlaylistsEntry>>, errors::Error> {
	let playlist_names = playlist::list_playlists(&auth.username, db.deref().deref())?;
	let playlists: Vec<ListPlaylistsEntry> = playlist_names
		.into_iter()
		.map(|p| ListPlaylistsEntry { name: p })
		.collect();

	Ok(Json(playlists))
}

#[derive(Deserialize)]
struct SavePlaylistInput {
	tracks: Vec<String>,
}

#[put("/playlist/<name>", data = "<playlist>")]
fn save_playlist(
	db: State<Arc<DB>>,
	auth: Auth,
	name: String,
	playlist: Json<SavePlaylistInput>,
) -> Result<(), errors::Error> {
	playlist::save_playlist(&name, &auth.username, &playlist.tracks, db.deref().deref())?;
	Ok(())
}

#[get("/playlist/<name>")]
fn read_playlist(
	db: State<Arc<DB>>,
	auth: Auth,
	name: String,
) -> Result<Json<Vec<index::Song>>, errors::Error> {
	let songs = playlist::read_playlist(&name, &auth.username, db.deref().deref())?;
	Ok(Json(songs))
}

#[delete("/playlist/<name>")]
fn delete_playlist(db: State<Arc<DB>>, auth: Auth, name: String) -> Result<(), errors::Error> {
	playlist::delete_playlist(&name, &auth.username, db.deref().deref())?;
	Ok(())
}

#[put("/lastfm/now_playing/<path>")]
fn lastfm_now_playing(
	db: State<Arc<DB>>,
	auth: Auth,
	path: VFSPathBuf,
) -> Result<(), errors::Error> {
	lastfm::now_playing(db.deref().deref(), &auth.username, &path.into() as &PathBuf)?;
	Ok(())
}

#[post("/lastfm/scrobble/<path>")]
fn lastfm_scrobble(db: State<Arc<DB>>, auth: Auth, path: VFSPathBuf) -> Result<(), errors::Error> {
	lastfm::scrobble(db.deref().deref(), &auth.username, &path.into() as &PathBuf)?;
	Ok(())
}

#[get("/lastfm/link?<token>&<content>")]
fn lastfm_link(
	db: State<Arc<DB>>,
	auth: Auth,
	token: String,
	content: String,
) -> Result<Html<String>, errors::Error> {
	lastfm::link(db.deref().deref(), &auth.username, &token)?;

	// Percent decode
	let base64_content = match RawStr::from_str(&content).percent_decode() {
		Ok(s) => s,
		Err(_) => return Err(errors::Error::from(errors::ErrorKind::EncodingError).into()),
	};

	// Base64 decode
	let popup_content = match base64::decode(base64_content.as_bytes()) {
		Ok(c) => c,
		Err(_) => return Err(errors::Error::from(errors::ErrorKind::EncodingError).into()),
	};

	// UTF-8 decode
	let popup_content_string = match str::from_utf8(&popup_content) {
		Ok(s) => s,
		Err(_) => return Err(errors::Error::from(errors::ErrorKind::EncodingError).into()),
	};

	Ok(Html(popup_content_string.to_string()))
}

#[delete("/lastfm/link")]
fn lastfm_unlink(db: State<Arc<DB>>, auth: Auth) -> Result<(), errors::Error> {
	lastfm::unlink(db.deref().deref(), &auth.username)?;
	Ok(())
}
