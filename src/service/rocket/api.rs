use anyhow::*;
use rocket::http::{Cookie, Cookies, RawStr, Status};
use rocket::request::{self, FromParam, FromRequest, Request};
use rocket::response::content::Html;
use rocket::{delete, get, post, put, routes, Outcome, State};
use rocket_contrib::json::Json;
use std::default::Default;
use std::fs::File;
use std::ops::Deref;
use std::path::PathBuf;
use std::str;
use std::str::FromStr;
use time::Duration;

use super::serve;
use crate::config::{self, Config, Preferences};
use crate::db::DB;
use crate::index;
use crate::index::Index;
use crate::lastfm;
use crate::playlist;
use crate::service::constants::*;
use crate::service::dto;
use crate::service::error::APIError;
use crate::thumbnails::{ThumbnailOptions, ThumbnailsManager};
use crate::user;
use crate::vfs::VFSSource;

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
		audio,
		thumbnail,
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

impl<'r> rocket::response::Responder<'r> for APIError {
	fn respond_to(self, _: &rocket::request::Request<'_>) -> rocket::response::Result<'r> {
		let status = match self {
			APIError::IncorrectCredentials => rocket::http::Status::Unauthorized,
			_ => rocket::http::Status::InternalServerError,
		};
		rocket::response::Response::build().status(status).ok()
	}
}

struct Auth {
	username: String,
}

fn add_session_cookies(cookies: &mut Cookies, username: &str, is_admin: bool) -> () {
	let duration = Duration::days(1);

	let session_cookie = Cookie::build(COOKIE_SESSION, username.to_owned())
		.same_site(rocket::http::SameSite::Lax)
		.http_only(true)
		.max_age(duration)
		.finish();

	let username_cookie = Cookie::build(COOKIE_USERNAME, username.to_owned())
		.same_site(rocket::http::SameSite::Lax)
		.http_only(false)
		.max_age(duration)
		.path("/")
		.finish();

	let is_admin_cookie = Cookie::build(COOKIE_ADMIN, format!("{}", is_admin))
		.same_site(rocket::http::SameSite::Lax)
		.http_only(false)
		.max_age(duration)
		.path("/")
		.finish();

	cookies.add_private(session_cookie);
	cookies.add(username_cookie);
	cookies.add(is_admin_cookie);
}

impl<'a, 'r> FromRequest<'a, 'r> for Auth {
	type Error = ();

	fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, ()> {
		let mut cookies = request.guard::<Cookies<'_>>().unwrap();
		let db = match request.guard::<State<'_, DB>>() {
			Outcome::Success(d) => d,
			_ => return Outcome::Failure((Status::InternalServerError, ())),
		};

		if let Some(u) = cookies.get_private(COOKIE_SESSION) {
			let exists = match user::exists(db.deref().deref(), u.value()) {
				Ok(e) => e,
				Err(_) => return Outcome::Failure((Status::InternalServerError, ())),
			};
			if !exists {
				return Outcome::Failure((Status::Unauthorized, ()));
			}
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
				if user::auth(db.deref().deref(), &username, &password).unwrap_or(false) {
					let is_admin = match user::is_admin(db.deref().deref(), &username) {
						Ok(a) => a,
						Err(_) => return Outcome::Failure((Status::InternalServerError, ())),
					};
					add_session_cookies(&mut cookies, &username, is_admin);
					return Outcome::Success(Auth {
						username: username.to_string(),
					});
				}
			}
		}

		Outcome::Failure((Status::Unauthorized, ()))
	}
}

struct AdminRights {
	auth: Option<Auth>,
}

impl<'a, 'r> FromRequest<'a, 'r> for AdminRights {
	type Error = ();

	fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, ()> {
		let db = request.guard::<State<'_, DB>>()?;

		match user::count(&db) {
			Err(_) => return Outcome::Failure((Status::InternalServerError, ())),
			Ok(0) => return Outcome::Success(AdminRights { auth: None }),
			_ => (),
		};

		let auth = request.guard::<Auth>()?;
		match user::is_admin(&db, &auth.username) {
			Err(_) => Outcome::Failure((Status::InternalServerError, ())),
			Ok(true) => Outcome::Success(AdminRights { auth: Some(auth) }),
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

#[get("/version")]
fn version() -> Json<dto::Version> {
	let current_version = dto::Version {
		major: API_MAJOR_VERSION,
		minor: API_MINOR_VERSION,
	};
	Json(current_version)
}

#[get("/initial_setup")]
fn initial_setup(db: State<'_, DB>) -> Result<Json<dto::InitialSetup>> {
	let initial_setup = dto::InitialSetup {
		has_any_users: user::count(&db)? > 0,
	};
	Ok(Json(initial_setup))
}

#[get("/settings")]
fn get_settings(db: State<'_, DB>, _admin_rights: AdminRights) -> Result<Json<Config>> {
	let config = config::read(&db)?;
	Ok(Json(config))
}

#[put("/settings", data = "<config>")]
fn put_settings(db: State<'_, DB>, admin_rights: AdminRights, config: Json<Config>) -> Result<()> {
	// Do not let users remove their own admin rights
	let mut sanitized_config = config.clone();
	if let Some(users) = &mut sanitized_config.users {
		for user in users.iter_mut() {
			if let Some(auth) = &admin_rights.auth {
				if auth.username == user.name {
					user.admin = true;
				}
			}
		}
	}
	config::amend(&db, &sanitized_config)?;
	Ok(())
}

#[get("/preferences")]
fn get_preferences(db: State<'_, DB>, auth: Auth) -> Result<Json<Preferences>> {
	let preferences = config::read_preferences(&db, &auth.username)?;
	Ok(Json(preferences))
}

#[put("/preferences", data = "<preferences>")]
fn put_preferences(db: State<'_, DB>, auth: Auth, preferences: Json<Preferences>) -> Result<()> {
	config::write_preferences(&db, &auth.username, &preferences)?;
	Ok(())
}

#[post("/trigger_index")]
fn trigger_index(index: State<'_, Index>, _admin_rights: AdminRights) -> Result<()> {
	index.trigger_reindex();
	Ok(())
}

#[post("/auth", data = "<credentials>")]
fn auth(
	db: State<'_, DB>,
	credentials: Json<dto::AuthCredentials>,
	mut cookies: Cookies<'_>,
) -> std::result::Result<(), APIError> {
	if !user::auth(&db, &credentials.username, &credentials.password)? {
		return Err(APIError::IncorrectCredentials);
	}
	let is_admin = user::is_admin(&db, &credentials.username)?;
	add_session_cookies(&mut cookies, &credentials.username, is_admin);
	Ok(())
}

#[get("/browse")]
fn browse_root(db: State<'_, DB>, _auth: Auth) -> Result<Json<Vec<index::CollectionFile>>> {
	let result = index::browse(db.deref().deref(), &PathBuf::new())?;
	Ok(Json(result))
}

#[get("/browse/<path>")]
fn browse(
	db: State<'_, DB>,
	_auth: Auth,
	path: VFSPathBuf,
) -> Result<Json<Vec<index::CollectionFile>>> {
	let result = index::browse(db.deref().deref(), &path.into() as &PathBuf)?;
	Ok(Json(result))
}

#[get("/flatten")]
fn flatten_root(db: State<'_, DB>, _auth: Auth) -> Result<Json<Vec<index::Song>>> {
	let result = index::flatten(db.deref().deref(), &PathBuf::new())?;
	Ok(Json(result))
}

#[get("/flatten/<path>")]
fn flatten(db: State<'_, DB>, _auth: Auth, path: VFSPathBuf) -> Result<Json<Vec<index::Song>>> {
	let result = index::flatten(db.deref().deref(), &path.into() as &PathBuf)?;
	Ok(Json(result))
}

#[get("/random")]
fn random(db: State<'_, DB>, _auth: Auth) -> Result<Json<Vec<index::Directory>>> {
	let result = index::get_random_albums(db.deref().deref(), 20)?;
	Ok(Json(result))
}

#[get("/recent")]
fn recent(db: State<'_, DB>, _auth: Auth) -> Result<Json<Vec<index::Directory>>> {
	let result = index::get_recent_albums(db.deref().deref(), 20)?;
	Ok(Json(result))
}

#[get("/search")]
fn search_root(db: State<'_, DB>, _auth: Auth) -> Result<Json<Vec<index::CollectionFile>>> {
	let result = index::search(db.deref().deref(), "")?;
	Ok(Json(result))
}

#[get("/search/<query>")]
fn search(
	db: State<'_, DB>,
	_auth: Auth,
	query: String,
) -> Result<Json<Vec<index::CollectionFile>>> {
	let result = index::search(db.deref().deref(), &query)?;
	Ok(Json(result))
}

#[get("/audio/<path>")]
fn audio(db: State<'_, DB>, _auth: Auth, path: VFSPathBuf) -> Result<serve::RangeResponder<File>> {
	let vfs = db.get_vfs()?;
	let real_path = vfs.virtual_to_real(&path.into() as &PathBuf)?;
	let file = File::open(&real_path)?;
	Ok(serve::RangeResponder::new(file))
}

#[get("/thumbnail/<path>?<pad>")]
fn thumbnail(
	db: State<'_, DB>,
	thumbnails_manager: State<'_, ThumbnailsManager>,
	_auth: Auth,
	path: VFSPathBuf,
	pad: Option<bool>,
) -> Result<File> {
	let vfs = db.get_vfs()?;
	let image_path = vfs.virtual_to_real(&path.into() as &PathBuf)?;
	let mut options = ThumbnailOptions::default();
	options.pad_to_square = pad.unwrap_or(options.pad_to_square);
	let thumbnail_path = thumbnails_manager.get_thumbnail(&image_path, &options)?;
	let file = File::open(thumbnail_path)?;
	Ok(file)
}

#[get("/playlists")]
fn list_playlists(db: State<'_, DB>, auth: Auth) -> Result<Json<Vec<dto::ListPlaylistsEntry>>> {
	let playlist_names = playlist::list_playlists(&auth.username, db.deref().deref())?;
	let playlists: Vec<dto::ListPlaylistsEntry> = playlist_names
		.into_iter()
		.map(|p| dto::ListPlaylistsEntry { name: p })
		.collect();

	Ok(Json(playlists))
}

#[put("/playlist/<name>", data = "<playlist>")]
fn save_playlist(
	db: State<'_, DB>,
	auth: Auth,
	name: String,
	playlist: Json<dto::SavePlaylistInput>,
) -> Result<()> {
	playlist::save_playlist(&name, &auth.username, &playlist.tracks, db.deref().deref())?;
	Ok(())
}

#[get("/playlist/<name>")]
fn read_playlist(db: State<'_, DB>, auth: Auth, name: String) -> Result<Json<Vec<index::Song>>> {
	let songs = playlist::read_playlist(&name, &auth.username, db.deref().deref())?;
	Ok(Json(songs))
}

#[delete("/playlist/<name>")]
fn delete_playlist(db: State<'_, DB>, auth: Auth, name: String) -> Result<()> {
	playlist::delete_playlist(&name, &auth.username, db.deref().deref())?;
	Ok(())
}

#[put("/lastfm/now_playing/<path>")]
fn lastfm_now_playing(db: State<'_, DB>, auth: Auth, path: VFSPathBuf) -> Result<()> {
	if user::is_lastfm_linked(db.deref().deref(), &auth.username) {
		lastfm::now_playing(db.deref().deref(), &auth.username, &path.into() as &PathBuf)?;
	}
	Ok(())
}

#[post("/lastfm/scrobble/<path>")]
fn lastfm_scrobble(db: State<'_, DB>, auth: Auth, path: VFSPathBuf) -> Result<()> {
	if user::is_lastfm_linked(db.deref().deref(), &auth.username) {
		lastfm::scrobble(db.deref().deref(), &auth.username, &path.into() as &PathBuf)?;
	}
	Ok(())
}

#[get("/lastfm/link?<token>&<content>")]
fn lastfm_link(
	db: State<'_, DB>,
	auth: Auth,
	token: String,
	content: String,
) -> Result<Html<String>> {
	lastfm::link(db.deref().deref(), &auth.username, &token)?;

	// Percent decode
	let base64_content = RawStr::from_str(&content).percent_decode()?;

	// Base64 decode
	let popup_content = base64::decode(base64_content.as_bytes())?;

	// UTF-8 decode
	let popup_content_string = str::from_utf8(&popup_content)?;

	Ok(Html(popup_content_string.to_string()))
}

#[delete("/lastfm/link")]
fn lastfm_unlink(db: State<'_, DB>, auth: Auth) -> Result<()> {
	lastfm::unlink(db.deref().deref(), &auth.username)?;
	Ok(())
}
