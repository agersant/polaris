use rocket::http::{Cookie, Cookies, RawStr, Status};
use rocket::request::{self, FromParam, FromRequest, Request};
use rocket::response::content::Html;
use rocket::{Outcome, State};
use rocket_contrib::json::Json;
use std::fs::File;
use std::ops::Deref;
use std::path::PathBuf;
use std::str;
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

impl<'a, 'r> FromRequest<'a, 'r> for Auth {
	type Error = ();

	fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, ()> {
		let mut cookies = request.guard::<Cookies>().unwrap();
		match cookies.get_private(SESSION_FIELD_USERNAME) {
			Some(u) => Outcome::Success(Auth {
				username: u.value().to_string(),
			}),
			_ => Outcome::Failure((Status::Forbidden, ())),
		}

		// TODO allow auth via authorization header
	}
}

struct AdminRights {}
impl<'a, 'r> FromRequest<'a, 'r> for AdminRights {
	type Error = ();

	fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, ()> {
		let db = request.guard::<State<DB>>()?;

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
fn initial_setup(db: State<DB>) -> Result<Json<InitialSetup>, errors::Error> {
	let initial_setup = InitialSetup {
		has_any_users: user::count::<DB>(&db)? > 0,
	};
	Ok(Json(initial_setup))
}

#[get("/settings")]
fn get_settings(db: State<DB>, _admin_rights: AdminRights) -> Result<Json<Config>, errors::Error> {
	let config = config::read::<DB>(&db)?;
	Ok(Json(config))
}

#[put("/settings", data = "<config>")]
fn put_settings(
	db: State<DB>,
	_admin_rights: AdminRights,
	config: Json<Config>,
) -> Result<(), errors::Error> {
	config::amend::<DB>(&db, &config)?;
	Ok(())
}

#[get("/preferences")]
fn get_preferences(db: State<DB>, auth: Auth) -> Result<Json<Preferences>, errors::Error> {
	let preferences = config::read_preferences::<DB>(&db, &auth.username)?;
	Ok(Json(preferences))
}

#[put("/preferences", data = "<preferences>")]
fn put_preferences(
	db: State<DB>,
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
	db: State<DB>,
	credentials: Json<AuthCredentials>,
	mut cookies: Cookies,
) -> Result<Json<AuthOutput>, errors::Error> {
	user::auth::<DB>(&db, &credentials.username, &credentials.password)?;
	cookies.add_private(
		Cookie::build(SESSION_FIELD_USERNAME, credentials.username.clone())
			.same_site(rocket::http::SameSite::Lax)
			.finish(),
	);

	let auth_output = AuthOutput {
		admin: user::is_admin::<DB>(&db, &credentials.username)?,
	};
	Ok(Json(auth_output))
}

#[get("/browse")]
fn browse_root(
	db: State<DB>,
	_auth: Auth,
) -> Result<Json<Vec<index::CollectionFile>>, errors::Error> {
	let result = index::browse(db.deref(), &PathBuf::new())?;
	Ok(Json(result))
}

#[get("/browse/<path>")]
fn browse(
	db: State<DB>,
	_auth: Auth,
	path: VFSPathBuf,
) -> Result<Json<Vec<index::CollectionFile>>, errors::Error> {
	let result = index::browse(db.deref(), &path.into() as &PathBuf)?;
	Ok(Json(result))
}

#[get("/flatten")]
fn flatten_root(db: State<DB>, _auth: Auth) -> Result<Json<Vec<index::Song>>, errors::Error> {
	let result = index::flatten(db.deref(), &PathBuf::new())?;
	Ok(Json(result))
}

#[get("/flatten/<path>")]
fn flatten(
	db: State<DB>,
	_auth: Auth,
	path: VFSPathBuf,
) -> Result<Json<Vec<index::Song>>, errors::Error> {
	let result = index::flatten(db.deref(), &path.into() as &PathBuf)?;
	Ok(Json(result))
}

#[get("/random")]
fn random(db: State<DB>, _auth: Auth) -> Result<Json<Vec<index::Directory>>, errors::Error> {
	let result = index::get_random_albums(db.deref(), 20)?;
	Ok(Json(result))
}

#[get("/recent")]
fn recent(db: State<DB>, _auth: Auth) -> Result<Json<Vec<index::Directory>>, errors::Error> {
	let result = index::get_recent_albums(db.deref(), 20)?;
	Ok(Json(result))
}

#[get("/search")]
fn search_root(
	db: State<DB>,
	_auth: Auth,
) -> Result<Json<Vec<index::CollectionFile>>, errors::Error> {
	let result = index::search(db.deref(), "")?;
	Ok(Json(result))
}

#[get("/search/<query>")]
fn search(
	db: State<DB>,
	_auth: Auth,
	query: String,
) -> Result<Json<Vec<index::CollectionFile>>, errors::Error> {
	let result = index::search(db.deref(), &query)?;
	Ok(Json(result))
}

#[get("/serve/<path>")]
fn serve(
	db: State<DB>,
	_auth: Auth,
	path: VFSPathBuf,
) -> Result<serve::RangeResponder<File>, errors::Error> {
	let db: &DB = db.deref();
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
	db: State<DB>,
	auth: Auth,
) -> Result<Json<Vec<ListPlaylistsEntry>>, errors::Error> {
	let playlist_names = playlist::list_playlists(&auth.username, db.deref())?;
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
	db: State<DB>,
	auth: Auth,
	name: String,
	playlist: Json<SavePlaylistInput>,
) -> Result<(), errors::Error> {
	playlist::save_playlist(&name, &auth.username, &playlist.tracks, db.deref())?;
	Ok(())
}

#[get("/playlist/<name>")]
fn read_playlist(
	db: State<DB>,
	auth: Auth,
	name: String,
) -> Result<Json<Vec<index::Song>>, errors::Error> {
	let songs = playlist::read_playlist(&name, &auth.username, db.deref())?;
	Ok(Json(songs))
}

#[delete("/playlist/<name>")]
fn delete_playlist(db: State<DB>, auth: Auth, name: String) -> Result<(), errors::Error> {
	playlist::delete_playlist(&name, &auth.username, db.deref())?;
	Ok(())
}

#[put("/lastfm/now_playing/<path>")]
fn lastfm_now_playing(db: State<DB>, auth: Auth, path: VFSPathBuf) -> Result<(), errors::Error> {
	lastfm::now_playing(db.deref(), &auth.username, &path.into() as &PathBuf)?;
	Ok(())
}

#[post("/lastfm/scrobble/<path>")]
fn lastfm_scrobble(db: State<DB>, auth: Auth, path: VFSPathBuf) -> Result<(), errors::Error> {
	lastfm::scrobble(db.deref(), &auth.username, &path.into() as &PathBuf)?;
	Ok(())
}

#[get("/lastfm/link?<token>&<content>")]
fn lastfm_link(
	db: State<DB>,
	auth: Auth,
	token: String,
	content: String,
) -> Result<Html<String>, errors::Error> {
	lastfm::link(db.deref(), &auth.username, &token)?;

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
fn lastfm_unlink(db: State<DB>, auth: Auth) -> Result<(), errors::Error> {
	lastfm::unlink(db.deref(), &auth.username)?;
	Ok(())
}
