use actix_files::NamedFile;
use actix_web::error::{ErrorForbidden, ErrorInternalServerError, ErrorUnauthorized};
use actix_web::{
	delete, dev::HttpResponseBuilder, dev::Payload, get, http::StatusCode, post, put, web,
	web::Data, web::Json, web::ServiceConfig, FromRequest, HttpMessage, HttpRequest, HttpResponse,
	ResponseError,
};
use actix_web_httpauth::extractors::basic::BasicAuth;
use cookie::{self, Cookie};
use futures_util::future::{err, ok};
use percent_encoding::percent_decode_str;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::str;

use crate::config::{self, Config, Preferences};
use crate::db::DB;
use crate::index::{self, Index};
use crate::lastfm;
use crate::playlist;
use crate::service::{dto, error::*};
use crate::thumbnails::{ThumbnailOptions, ThumbnailsManager};
use crate::user;
use crate::vfs::VFSSource;

// TODO Use async instead of long blocking operations:
// - Thumnbail generation
// - Everything that touches DB

pub fn make_config() -> impl FnOnce(&mut ServiceConfig) + Clone {
	move |cfg: &mut ServiceConfig| {
		cfg.service(version)
			.service(initial_setup)
			.service(get_settings)
			.service(put_settings)
			.service(get_preferences)
			.service(put_preferences)
			.service(trigger_index)
			.service(login)
			.service(browse)
			.service(flatten)
			.service(random)
			.service(recent)
			.service(search)
			.service(audio)
			.service(thumbnail)
			.service(list_playlists)
			.service(save_playlist)
			.service(read_playlist)
			.service(delete_playlist)
			.service(lastfm_now_playing)
			.service(lastfm_scrobble)
			.service(lastfm_link)
			.service(lastfm_unlink);
	}
}

impl ResponseError for APIError {
	fn status_code(&self) -> StatusCode {
		match self {
			APIError::IncorrectCredentials => StatusCode::UNAUTHORIZED,
			APIError::OwnAdminPrivilegeRemoval => StatusCode::CONFLICT,
			APIError::AudioFileIOError => StatusCode::NOT_FOUND,
			APIError::ThumbnailFileIOError => StatusCode::NOT_FOUND,
			APIError::LastFMLinkContentBase64DecodeError => StatusCode::BAD_REQUEST,
			APIError::LastFMLinkContentEncodingError => StatusCode::BAD_REQUEST,
			APIError::UserNotFound => StatusCode::NOT_FOUND,
			APIError::PlaylistNotFound => StatusCode::NOT_FOUND,
			APIError::VFSPathNotFound => StatusCode::NOT_FOUND,
			APIError::Unspecified => StatusCode::INTERNAL_SERVER_ERROR,
		}
	}
}

#[derive(Debug)]
struct Auth {
	username: String,
}

impl FromRequest for Auth {
	type Error = actix_web::Error;
	type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;
	type Config = ();

	fn from_request(request: &HttpRequest, payload: &mut Payload) -> Self::Future {
		let db = match request.app_data::<Data<DB>>() {
			Some(db) => db.clone(),
			None => return Box::pin(err(ErrorInternalServerError(APIError::Unspecified))),
		};

		if let Some(session_cookie) = request.cookie(dto::COOKIE_SESSION) {
			let exists = match user::exists(&db, session_cookie.value()) {
				Ok(e) => e,
				Err(_) => return Box::pin(err(ErrorInternalServerError(APIError::Unspecified))),
			};
			if !exists {
				return Box::pin(err(ErrorUnauthorized(APIError::Unspecified)));
			}
			return Box::pin(ok(Auth {
				username: session_cookie.value().to_owned(),
			}));
		}

		let auth_future = BasicAuth::from_request(request, payload);
		Box::pin(async move {
			let auth = auth_future.await?;
			let username = auth.user_id().as_ref();
			let password = auth.password().map(|s| s.as_ref()).unwrap_or("");
			if user::auth(&db, username, password).unwrap_or(false) {
				return Ok(Auth {
					username: username.to_owned(),
				});
			}
			Err(ErrorUnauthorized(APIError::Unspecified))
		})
	}
}

#[derive(Debug)]
struct AdminRights {
	auth: Option<Auth>,
}

impl FromRequest for AdminRights {
	type Error = actix_web::Error;
	type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;
	type Config = ();

	fn from_request(request: &HttpRequest, payload: &mut Payload) -> Self::Future {
		let db = match request.app_data::<Data<DB>>() {
			Some(db) => db.clone(),
			None => return Box::pin(err(ErrorInternalServerError(APIError::Unspecified))),
		};

		match user::count(&db) {
			Err(_) => return Box::pin(err(ErrorInternalServerError(APIError::Unspecified))),
			Ok(0) => return Box::pin(ok(AdminRights { auth: None })),
			_ => (),
		};

		let auth_future = Auth::from_request(request, payload);

		Box::pin(async move {
			let auth = auth_future.await?;
			match user::is_admin(&db, &auth.username) {
				Err(_) => Err(ErrorInternalServerError(APIError::Unspecified)),
				Ok(true) => Ok(AdminRights { auth: Some(auth) }),
				Ok(false) => Err(ErrorForbidden(APIError::Unspecified)),
			}
		})
	}
}

// TODO needs to add session cookies in response to auth header (expected by mobile client!!)
fn add_session_cookies(builder: &mut HttpResponseBuilder, username: &str, is_admin: bool) -> () {
	let duration = time::Duration::days(1);

	let session_cookie = Cookie::build(dto::COOKIE_SESSION, username.to_owned())
		.same_site(cookie::SameSite::Lax)
		.http_only(true)
		.max_age(duration)
		.finish();

	let username_cookie = Cookie::build(dto::COOKIE_USERNAME, username.to_owned())
		.same_site(cookie::SameSite::Lax)
		.http_only(false)
		.max_age(duration)
		.path("/")
		.finish();

	let is_admin_cookie = Cookie::build(dto::COOKIE_ADMIN, format!("{}", is_admin))
		.same_site(cookie::SameSite::Lax)
		.http_only(false)
		.max_age(duration)
		.path("/")
		.finish();

	builder.cookie(session_cookie); // TODO should be private encrypted cookie
	builder.cookie(username_cookie);
	builder.cookie(is_admin_cookie);
}

#[get("/version")]
async fn version() -> Json<dto::Version> {
	let current_version = dto::Version {
		major: dto::API_MAJOR_VERSION,
		minor: dto::API_MINOR_VERSION,
	};
	Json(current_version)
}

#[get("/initial_setup")]
async fn initial_setup(db: Data<DB>) -> Result<Json<dto::InitialSetup>, APIError> {
	let initial_setup = dto::InitialSetup {
		has_any_users: user::count(&db)? > 0,
	};
	Ok(Json(initial_setup))
}

#[get("/settings")]
async fn get_settings(db: Data<DB>, _admin_rights: AdminRights) -> Result<Json<Config>, APIError> {
	let config = config::read(&db)?;
	Ok(Json(config))
}

#[put("/settings")]
async fn put_settings(
	admin_rights: AdminRights,
	db: Data<DB>,
	config: Json<Config>,
) -> Result<HttpResponse, APIError> {
	// TODO config should be a dto type

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

	config::amend(&db, &config)?;
	Ok(HttpResponse::new(StatusCode::OK))
}

#[get("/preferences")]
async fn get_preferences(db: Data<DB>, auth: Auth) -> Result<Json<Preferences>, APIError> {
	let preferences = config::read_preferences(&db, &auth.username)?;
	Ok(Json(preferences))
}

#[put("/preferences")]
async fn put_preferences(
	db: Data<DB>,
	auth: Auth,
	preferences: Json<Preferences>,
) -> Result<HttpResponse, APIError> {
	config::write_preferences(&db, &auth.username, &preferences)?;
	Ok(HttpResponse::new(StatusCode::OK))
}

#[post("/trigger_index")]
async fn trigger_index(
	index: Data<Index>,
	_admin_rights: AdminRights,
) -> Result<HttpResponse, APIError> {
	index.trigger_reindex();
	Ok(HttpResponse::new(StatusCode::OK))
}

#[post("/auth")]
async fn login(
	db: Data<DB>,
	credentials: Json<dto::AuthCredentials>,
) -> Result<HttpResponse, APIError> {
	if !user::auth(&db, &credentials.username, &credentials.password)? {
		return Err(APIError::IncorrectCredentials);
	}
	let is_admin = user::is_admin(&db, &credentials.username)?;
	let mut response = HttpResponse::Ok();
	add_session_cookies(&mut response, &credentials.username, is_admin);
	Ok(response.finish())
}

#[get("/browse/{path:.*}")]
async fn browse(
	db: Data<DB>,
	_auth: Auth,
	path: web::Path<PathBuf>,
) -> Result<Json<Vec<index::CollectionFile>>, APIError> {
	let result = index::browse(&db, &(path.0).into() as &PathBuf)?;
	Ok(Json(result))
}

#[get("/flatten/{path:.*}")]
async fn flatten(
	db: Data<DB>,
	_auth: Auth,
	path: web::Path<PathBuf>,
) -> Result<Json<Vec<index::Song>>, APIError> {
	let result = index::flatten(&db, &(path.0).into() as &PathBuf)?;
	Ok(Json(result))
}

#[get("/random")]
async fn random(db: Data<DB>, _auth: Auth) -> Result<Json<Vec<index::Directory>>, APIError> {
	let result = index::get_random_albums(&db, 20)?;
	Ok(Json(result))
}

#[get("/recent")]
async fn recent(db: Data<DB>, _auth: Auth) -> Result<Json<Vec<index::Directory>>, APIError> {
	let result = index::get_recent_albums(&db, 20)?;
	Ok(Json(result))
}

#[get("/search/{query:.*}")]
async fn search(
	db: Data<DB>,
	_auth: Auth,
	query: web::Path<String>,
) -> Result<Json<Vec<index::CollectionFile>>, APIError> {
	let result = index::search(&db, &query)?;
	Ok(Json(result))
}

#[get("/audio/{path:.*}")]
async fn audio(db: Data<DB>, _auth: Auth, path: web::Path<PathBuf>) -> Result<NamedFile, APIError> {
	let vfs = db.get_vfs()?;
	let real_path = vfs
		.virtual_to_real(&(path.0).into() as &PathBuf)
		.map_err(|_| APIError::VFSPathNotFound)?;
	let named_file = NamedFile::open(&real_path).map_err(|_| APIError::AudioFileIOError)?;
	Ok(named_file)
}

#[get("/thumbnail/{path:.*}")]
async fn thumbnail(
	db: Data<DB>,
	thumbnails_manager: Data<ThumbnailsManager>,
	_auth: Auth,
	path: web::Path<PathBuf>,
	options_input: web::Query<dto::ThumbnailOptions>,
) -> Result<NamedFile, APIError> {
	let vfs = db.get_vfs()?;
	let image_path = vfs
		.virtual_to_real(&(path.0).into() as &PathBuf)
		.map_err(|_| APIError::VFSPathNotFound)?;
	let mut options = ThumbnailOptions::default();
	options.pad_to_square = options_input.pad.unwrap_or(options.pad_to_square);
	let thumbnail_path = thumbnails_manager.get_thumbnail(&image_path, &options)?;
	let named_file =
		NamedFile::open(&thumbnail_path).map_err(|_| APIError::ThumbnailFileIOError)?;
	Ok(named_file)
}

#[get("/playlists")]
async fn list_playlists(
	db: Data<DB>,
	auth: Auth,
) -> Result<Json<Vec<dto::ListPlaylistsEntry>>, APIError> {
	dbg!("AA");
	let playlist_names = playlist::list_playlists(&auth.username, &db)?;
	let playlists: Vec<dto::ListPlaylistsEntry> = playlist_names
		.into_iter()
		.map(|p| dto::ListPlaylistsEntry { name: p })
		.collect();

	Ok(Json(playlists))
}

#[put("/playlist/{name}")]
async fn save_playlist(
	db: Data<DB>,
	auth: Auth,
	name: web::Path<String>,
	playlist: Json<dto::SavePlaylistInput>,
) -> Result<HttpResponse, APIError> {
	playlist::save_playlist(&name, &auth.username, &playlist.tracks, &db)?;
	Ok(HttpResponse::new(StatusCode::OK))
}

#[get("/playlist/{name}")]
async fn read_playlist(
	db: Data<DB>,
	auth: Auth,
	name: web::Path<String>,
) -> Result<Json<Vec<index::Song>>, APIError> {
	let songs = playlist::read_playlist(&name, &auth.username, &db)?;
	Ok(Json(songs))
}

#[delete("/playlist/{name}")]
async fn delete_playlist(
	db: Data<DB>,
	auth: Auth,
	name: web::Path<String>,
) -> Result<HttpResponse, APIError> {
	playlist::delete_playlist(&name, &auth.username, &db)?;
	Ok(HttpResponse::new(StatusCode::OK))
}

#[put("/lastfm/now_playing/<path>")]
async fn lastfm_now_playing(
	db: Data<DB>,
	auth: Auth,
	path: web::Path<String>,
) -> Result<HttpResponse, APIError> {
	if user::is_lastfm_linked(&db, &auth.username) {
		lastfm::now_playing(&db, &auth.username, &(path.0).into() as &PathBuf)?;
	}
	Ok(HttpResponse::new(StatusCode::OK))
}

#[post("/lastfm/scrobble/<path>")]
async fn lastfm_scrobble(
	db: Data<DB>,
	auth: Auth,
	path: web::Path<String>,
) -> Result<HttpResponse, APIError> {
	if user::is_lastfm_linked(&db, &auth.username) {
		lastfm::scrobble(&db, &auth.username, &(path.0).into() as &PathBuf)?;
	}
	Ok(HttpResponse::new(StatusCode::OK))
}

#[get("/lastfm/link")]
async fn lastfm_link(
	db: Data<DB>,
	auth: Auth,
	payload: web::Query<dto::LastFMLink>,
) -> Result<HttpResponse, APIError> {
	lastfm::link(&db, &auth.username, &payload.token)?;

	// Percent decode
	let base64_content = percent_decode_str(&payload.content).decode_utf8_lossy();

	// Base64 decode
	let popup_content = base64::decode(base64_content.as_bytes())
		.map_err(|_| APIError::LastFMLinkContentBase64DecodeError)?;

	// UTF-8 decode
	let popup_content_string =
		str::from_utf8(&popup_content).map_err(|_| APIError::LastFMLinkContentEncodingError)?;

	Ok(HttpResponse::build(StatusCode::OK)
		.content_type("text/html; charset=utf-8")
		.body(popup_content_string.to_owned()))
}

#[delete("/lastfm/link")]
async fn lastfm_unlink(db: Data<DB>, auth: Auth) -> Result<HttpResponse, APIError> {
	lastfm::unlink(&db, &auth.username)?;
	Ok(HttpResponse::new(StatusCode::OK))
}
