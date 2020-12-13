use anyhow::*;
use rocket::http::{Cookie, Cookies, RawStr, Status};
use rocket::request::{self, FromParam, FromRequest, Request};
use rocket::response::content::Html;
use rocket::{delete, get, post, put, routes, Outcome, State};
use rocket_contrib::json::Json;
use std::default::Default;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::str;
use std::str::FromStr;
use time::Duration;

use super::serve;
use crate::app::index::{self, Index, QueryError};
use crate::app::{config, lastfm, playlist, thumbnail, user, vfs};
use crate::service::dto;
use crate::service::error::APIError;

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
			APIError::OwnAdminPrivilegeRemoval => rocket::http::Status::Conflict,
			APIError::VFSPathNotFound => rocket::http::Status::NotFound,
			APIError::UserNotFound => rocket::http::Status::NotFound,
			APIError::PlaylistNotFound => rocket::http::Status::NotFound,
			APIError::Unspecified => rocket::http::Status::InternalServerError,
		};
		rocket::response::Response::build().status(status).ok()
	}
}

impl From<playlist::Error> for APIError {
	fn from(error: playlist::Error) -> APIError {
		match error {
			playlist::Error::PlaylistNotFound => APIError::PlaylistNotFound,
			playlist::Error::UserNotFound => APIError::UserNotFound,
			playlist::Error::Unspecified => APIError::Unspecified,
		}
	}
}

impl From<QueryError> for APIError {
	fn from(error: QueryError) -> APIError {
		match error {
			QueryError::VFSPathNotFound => APIError::VFSPathNotFound,
			QueryError::Unspecified => APIError::Unspecified,
		}
	}
}

struct Auth {
	username: String,
}

fn add_session_cookies(cookies: &mut Cookies, username: &str, is_admin: bool) -> () {
	let duration = Duration::days(1);

	let session_cookie = Cookie::build(dto::COOKIE_SESSION, username.to_owned())
		.same_site(rocket::http::SameSite::Lax)
		.http_only(true)
		.max_age(duration)
		.finish();

	let username_cookie = Cookie::build(dto::COOKIE_USERNAME, username.to_owned())
		.same_site(rocket::http::SameSite::Lax)
		.http_only(false)
		.max_age(duration)
		.path("/")
		.finish();

	let is_admin_cookie = Cookie::build(dto::COOKIE_ADMIN, format!("{}", is_admin))
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
		let user_manager = match request.guard::<State<'_, user::Manager>>() {
			Outcome::Success(d) => d,
			_ => return Outcome::Failure((Status::InternalServerError, ())),
		};

		if let Some(u) = cookies.get_private(dto::COOKIE_SESSION) {
			let exists = match user_manager.exists(u.value()) {
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
				if user_manager.auth(&username, &password).unwrap_or(false) {
					let is_admin = match user_manager.is_admin(&username) {
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
		let user_manager = request.guard::<State<'_, user::Manager>>()?;

		match user_manager.count() {
			Err(_) => return Outcome::Failure((Status::InternalServerError, ())),
			Ok(0) => return Outcome::Success(AdminRights { auth: None }),
			_ => (),
		};

		let auth = request.guard::<Auth>()?;
		match user_manager.is_admin(&auth.username) {
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
		major: dto::API_MAJOR_VERSION,
		minor: dto::API_MINOR_VERSION,
	};
	Json(current_version)
}

#[get("/initial_setup")]
fn initial_setup(user_manager: State<'_, user::Manager>) -> Result<Json<dto::InitialSetup>> {
	let initial_setup = dto::InitialSetup {
		has_any_users: user_manager.count()? > 0,
	};
	Ok(Json(initial_setup))
}

#[get("/settings")]
fn get_settings(
	config_manager: State<'_, config::Manager>,
	_admin_rights: AdminRights,
) -> Result<Json<config::Config>> {
	let config = config_manager.read()?;
	Ok(Json(config))
}

#[put("/settings", data = "<config>")]
fn put_settings(
	config_manager: State<'_, config::Manager>,
	admin_rights: AdminRights,
	config: Json<config::Config>,
) -> Result<(), APIError> {
	// Do not let users remove their own admin rights
	if let Some(auth) = &admin_rights.auth {
		if let Some(users) = &config.users {
			for user in users {
				if auth.username == user.name && !user.admin {
					return Err(APIError::OwnAdminPrivilegeRemoval);
				}
			}
		}
	}

	config_manager.amend(&config)?;
	Ok(())
}

#[get("/preferences")]
fn get_preferences(
	user_manager: State<'_, user::Manager>,
	auth: Auth,
) -> Result<Json<user::Preferences>> {
	let preferences = user_manager.read_preferences(&auth.username)?;
	Ok(Json(preferences))
}

#[put("/preferences", data = "<preferences>")]
fn put_preferences(
	user_manager: State<'_, user::Manager>,
	auth: Auth,
	preferences: Json<user::Preferences>,
) -> Result<()> {
	user_manager.write_preferences(&auth.username, &preferences)?;
	Ok(())
}

#[post("/trigger_index")]
fn trigger_index(index: State<'_, Index>, _admin_rights: AdminRights) -> Result<()> {
	index.trigger_reindex();
	Ok(())
}

#[post("/auth", data = "<credentials>")]
fn auth(
	user_manager: State<'_, user::Manager>,
	credentials: Json<dto::AuthCredentials>,
	mut cookies: Cookies<'_>,
) -> std::result::Result<(), APIError> {
	if !user_manager.auth(&credentials.username, &credentials.password)? {
		return Err(APIError::IncorrectCredentials);
	}
	let is_admin = user_manager.is_admin(&credentials.username)?;
	add_session_cookies(&mut cookies, &credentials.username, is_admin);
	Ok(())
}

#[get("/browse")]
fn browse_root(index: State<'_, Index>, _auth: Auth) -> Result<Json<Vec<index::CollectionFile>>> {
	let result = index.browse(&Path::new(""))?;
	Ok(Json(result))
}

#[get("/browse/<path>")]
fn browse(
	index: State<'_, Index>,
	_auth: Auth,
	path: VFSPathBuf,
) -> Result<Json<Vec<index::CollectionFile>>, APIError> {
	let result = index.browse(&path.into() as &PathBuf)?;
	Ok(Json(result))
}

#[get("/flatten")]
fn flatten_root(index: State<'_, Index>, _auth: Auth) -> Result<Json<Vec<index::Song>>> {
	let result = index.flatten(&PathBuf::new())?;
	Ok(Json(result))
}

#[get("/flatten/<path>")]
fn flatten(
	index: State<'_, Index>,
	_auth: Auth,
	path: VFSPathBuf,
) -> Result<Json<Vec<index::Song>>, APIError> {
	let result = index.flatten(&path.into() as &PathBuf)?;
	Ok(Json(result))
}

#[get("/random")]
fn random(index: State<'_, Index>, _auth: Auth) -> Result<Json<Vec<index::Directory>>> {
	let result = index.get_random_albums(20)?;
	Ok(Json(result))
}

#[get("/recent")]
fn recent(index: State<'_, Index>, _auth: Auth) -> Result<Json<Vec<index::Directory>>> {
	let result = index.get_recent_albums(20)?;
	Ok(Json(result))
}

#[get("/search")]
fn search_root(index: State<'_, Index>, _auth: Auth) -> Result<Json<Vec<index::CollectionFile>>> {
	let result = index.search("")?;
	Ok(Json(result))
}

#[get("/search/<query>")]
fn search(
	index: State<'_, Index>,
	_auth: Auth,
	query: String,
) -> Result<Json<Vec<index::CollectionFile>>> {
	let result = index.search(&query)?;
	Ok(Json(result))
}

#[get("/audio/<path>")]
fn audio(
	vfs_manager: State<'_, vfs::Manager>,
	_auth: Auth,
	path: VFSPathBuf,
) -> Result<serve::RangeResponder<File>, APIError> {
	let vfs = vfs_manager.get_vfs()?;
	let real_path = vfs
		.virtual_to_real(&path.into() as &PathBuf)
		.map_err(|_| APIError::VFSPathNotFound)?;
	let file = File::open(&real_path).map_err(|_| APIError::Unspecified)?;
	Ok(serve::RangeResponder::new(file))
}

#[get("/thumbnail/<path>?<pad>")]
fn thumbnail(
	vfs_manager: State<'_, vfs::Manager>,
	thumbnail_manager: State<'_, thumbnail::Manager>,
	_auth: Auth,
	path: VFSPathBuf,
	pad: Option<bool>,
) -> Result<File, APIError> {
	let vfs = vfs_manager.get_vfs()?;
	let image_path = vfs
		.virtual_to_real(&path.into() as &PathBuf)
		.map_err(|_| APIError::VFSPathNotFound)?;
	let mut options = thumbnail::Options::default();
	options.pad_to_square = pad.unwrap_or(options.pad_to_square);
	let thumbnail_path = thumbnail_manager.get_thumbnail(&image_path, &options)?;
	let file = File::open(thumbnail_path).map_err(|_| APIError::Unspecified)?;
	Ok(file)
}

#[get("/playlists")]
fn list_playlists(
	playlist_manager: State<'_, playlist::Manager>,
	auth: Auth,
) -> Result<Json<Vec<dto::ListPlaylistsEntry>>> {
	let playlist_names = playlist_manager.list_playlists(&auth.username)?;
	let playlists: Vec<dto::ListPlaylistsEntry> = playlist_names
		.into_iter()
		.map(|p| dto::ListPlaylistsEntry { name: p })
		.collect();

	Ok(Json(playlists))
}

#[put("/playlist/<name>", data = "<playlist>")]
fn save_playlist(
	playlist_manager: State<'_, playlist::Manager>,
	auth: Auth,
	name: String,
	playlist: Json<dto::SavePlaylistInput>,
) -> Result<()> {
	playlist_manager.save_playlist(&name, &auth.username, &playlist.tracks)?;
	Ok(())
}

#[get("/playlist/<name>")]
fn read_playlist(
	playlist_manager: State<'_, playlist::Manager>,
	auth: Auth,
	name: String,
) -> Result<Json<Vec<index::Song>>, APIError> {
	let songs = playlist_manager.read_playlist(&name, &auth.username)?;
	Ok(Json(songs))
}

#[delete("/playlist/<name>")]
fn delete_playlist(
	playlist_manager: State<'_, playlist::Manager>,
	auth: Auth,
	name: String,
) -> Result<(), APIError> {
	playlist_manager.delete_playlist(&name, &auth.username)?;
	Ok(())
}

#[put("/lastfm/now_playing/<path>")]
fn lastfm_now_playing(
	user_manager: State<'_, user::Manager>,
	lastfm_manager: State<'_, lastfm::Manager>,
	auth: Auth,
	path: VFSPathBuf,
) -> Result<()> {
	if user_manager.is_lastfm_linked(&auth.username) {
		lastfm_manager.now_playing(&auth.username, &path.into() as &PathBuf)?;
	}
	Ok(())
}

#[post("/lastfm/scrobble/<path>")]
fn lastfm_scrobble(
	user_manager: State<'_, user::Manager>,
	lastfm_manager: State<'_, lastfm::Manager>,
	auth: Auth,
	path: VFSPathBuf,
) -> Result<()> {
	if user_manager.is_lastfm_linked(&auth.username) {
		lastfm_manager.scrobble(&auth.username, &path.into() as &PathBuf)?;
	}
	Ok(())
}

#[get("/lastfm/link?<token>&<content>")]
fn lastfm_link(
	lastfm_manager: State<'_, lastfm::Manager>,
	auth: Auth,
	token: String,
	content: String,
) -> Result<Html<String>> {
	lastfm_manager.link(&auth.username, &token)?;

	// Percent decode
	let base64_content = RawStr::from_str(&content).percent_decode()?;

	// Base64 decode
	let popup_content = base64::decode(base64_content.as_bytes())?;

	// UTF-8 decode
	let popup_content_string = str::from_utf8(&popup_content)?;

	Ok(Html(popup_content_string.to_string()))
}

#[delete("/lastfm/link")]
fn lastfm_unlink(lastfm_manager: State<'_, lastfm::Manager>, auth: Auth) -> Result<()> {
	lastfm_manager.unlink(&auth.username)?;
	Ok(())
}
