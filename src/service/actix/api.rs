use actix_files::NamedFile;
use actix_web::body::BoxBody;
use actix_web::http::header::ContentEncoding;
use actix_web::{
	delete,
	dev::Payload,
	error::{ErrorForbidden, ErrorInternalServerError, ErrorUnauthorized},
	get,
	http::StatusCode,
	post, put,
	web::{self, Data, Json, JsonConfig, ServiceConfig},
	FromRequest, HttpRequest, HttpResponse, Responder, ResponseError,
};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use base64::prelude::*;
use futures_util::future::err;
use percent_encoding::percent_decode_str;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::str;

use crate::app::{
	config, ddns,
	index::{self, Index},
	lastfm, playlist, settings, thumbnail, user,
	vfs::{self, MountDir},
};
use crate::service::{dto, error::*};

pub fn make_config() -> impl FnOnce(&mut ServiceConfig) + Clone {
	move |cfg: &mut ServiceConfig| {
		let megabyte = 1024 * 1024;
		cfg.app_data(JsonConfig::default().limit(4 * megabyte)) // 4MB
			.service(version)
			.service(initial_setup)
			.service(apply_config)
			.service(get_settings)
			.service(put_settings)
			.service(list_mount_dirs)
			.service(put_mount_dirs)
			.service(get_ddns_config)
			.service(put_ddns_config)
			.service(list_users)
			.service(create_user)
			.service(update_user)
			.service(delete_user)
			.service(get_preferences)
			.service(put_preferences)
			.service(trigger_index)
			.service(login)
			.service(browse_root)
			.service(browse)
			.service(flatten_root)
			.service(flatten)
			.service(random)
			.service(recent)
			.service(search_root)
			.service(search)
			.service(get_audio)
			.service(get_thumbnail)
			.service(list_playlists)
			.service(save_playlist)
			.service(read_playlist)
			.service(delete_playlist)
			.service(lastfm_now_playing)
			.service(lastfm_scrobble)
			.service(lastfm_link_token)
			.service(lastfm_link)
			.service(lastfm_unlink);
	}
}

impl ResponseError for APIError {
	fn status_code(&self) -> StatusCode {
		match self {
			APIError::AuthorizationTokenEncoding => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::AdminPermissionRequired => StatusCode::UNAUTHORIZED,
			APIError::AudioFileIOError => StatusCode::NOT_FOUND,
			APIError::AuthenticationRequired => StatusCode::UNAUTHORIZED,
			APIError::BrancaTokenEncoding => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::DdnsUpdateQueryFailed(s) => {
				StatusCode::from_u16(*s).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
			}
			APIError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::DeletingOwnAccount => StatusCode::CONFLICT,
			APIError::EmbeddedArtworkNotFound => StatusCode::NOT_FOUND,
			APIError::EmptyPassword => StatusCode::BAD_REQUEST,
			APIError::EmptyUsername => StatusCode::BAD_REQUEST,
			APIError::IncorrectCredentials => StatusCode::UNAUTHORIZED,
			APIError::Internal => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::Io(_, _) => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::LastFMAccountNotLinked => StatusCode::NO_CONTENT,
			APIError::LastFMLinkContentBase64DecodeError => StatusCode::BAD_REQUEST,
			APIError::LastFMLinkContentEncodingError => StatusCode::BAD_REQUEST,
			APIError::LastFMNowPlaying(_) => StatusCode::FAILED_DEPENDENCY,
			APIError::LastFMScrobble(_) => StatusCode::FAILED_DEPENDENCY,
			APIError::LastFMScrobblerAuthentication(_) => StatusCode::FAILED_DEPENDENCY,
			APIError::OwnAdminPrivilegeRemoval => StatusCode::CONFLICT,
			APIError::PasswordHashing => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::PlaylistNotFound => StatusCode::NOT_FOUND,
			APIError::Settings(_) => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::SongMetadataNotFound => StatusCode::NOT_FOUND,
			APIError::ThumbnailFlacDecoding(_, _) => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::ThumbnailFileIOError => StatusCode::NOT_FOUND,
			APIError::ThumbnailId3Decoding(_, _) => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::ThumbnailImageDecoding(_, _) => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::ThumbnailMp4Decoding(_, _) => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::TomlDeserialization(_) => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::UnsupportedThumbnailFormat(_) => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::UserNotFound => StatusCode::NOT_FOUND,
			APIError::VFSPathNotFound => StatusCode::NOT_FOUND,
		}
	}

	fn error_response(&self) -> HttpResponse<BoxBody> {
		HttpResponse::new(self.status_code())
	}
}

#[derive(Debug)]
struct Auth {
	username: String,
}

impl FromRequest for Auth {
	type Error = actix_web::Error;
	type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

	fn from_request(request: &HttpRequest, payload: &mut Payload) -> Self::Future {
		let user_manager = match request.app_data::<Data<user::Manager>>() {
			Some(m) => m.clone(),
			None => return Box::pin(err(ErrorInternalServerError(APIError::Internal))),
		};

		let bearer_auth_future = BearerAuth::from_request(request, payload);
		let query_params_future =
			web::Query::<dto::AuthQueryParameters>::from_request(request, payload);

		Box::pin(async move {
			// Auth via bearer token in query parameter
			if let Ok(query) = query_params_future.await {
				let auth_token = user::AuthToken(query.auth_token.clone());
				let authorization = block(move || {
					user_manager.authenticate(&auth_token, user::AuthorizationScope::PolarisAuth)
				})
				.await?;
				return Ok(Auth {
					username: authorization.username,
				});
			}

			// Auth via bearer token in authorization header
			if let Ok(bearer_auth) = bearer_auth_future.await {
				let auth_token = user::AuthToken(bearer_auth.token().to_owned());
				let authorization = block(move || {
					user_manager.authenticate(&auth_token, user::AuthorizationScope::PolarisAuth)
				})
				.await?;
				return Ok(Auth {
					username: authorization.username,
				});
			}

			Err(ErrorUnauthorized(APIError::AuthenticationRequired))
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

	fn from_request(request: &HttpRequest, payload: &mut Payload) -> Self::Future {
		let user_manager = match request.app_data::<Data<user::Manager>>() {
			Some(m) => m.clone(),
			None => return Box::pin(err(ErrorInternalServerError(APIError::Internal))),
		};

		let auth_future = Auth::from_request(request, payload);

		Box::pin(async move {
			let user_manager_count = user_manager.clone();
			let user_count = block(move || user_manager_count.count()).await;
			match user_count {
				Err(e) => return Err(e.into()),
				Ok(0) => return Ok(AdminRights { auth: None }),
				_ => (),
			};

			let auth = auth_future.await?;
			let username = auth.username.clone();
			let is_admin = block(move || user_manager.is_admin(&username)).await?;
			if is_admin {
				Ok(AdminRights { auth: Some(auth) })
			} else {
				Err(ErrorForbidden(APIError::AdminPermissionRequired))
			}
		})
	}
}

struct MediaFile {
	named_file: NamedFile,
}

impl MediaFile {
	fn new(named_file: NamedFile) -> Self {
		Self { named_file }
	}
}

impl Responder for MediaFile {
	type Body = BoxBody;

	fn respond_to(self, req: &HttpRequest) -> HttpResponse<Self::Body> {
		// Intentionally turn off content encoding for media files because:
		// 1. There is little value in compressing files that are already compressed (mp3, jpg, etc.)
		// 2. The Content-Length header is incompatible with content encoding (other than identity), and can be valuable for clients
		self.named_file
			.set_content_encoding(ContentEncoding::Identity)
			.into_response(req)
	}
}

async fn block<F, I, E>(f: F) -> Result<I, APIError>
where
	F: FnOnce() -> Result<I, E> + Send + 'static,
	I: Send + 'static,
	E: Send + std::fmt::Debug + 'static + Into<APIError>,
{
	actix_web::web::block(f)
		.await
		.map_err(|_| APIError::Internal)
		.and_then(|r| r.map_err(|e| e.into()))
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
async fn initial_setup(
	user_manager: Data<user::Manager>,
) -> Result<Json<dto::InitialSetup>, APIError> {
	let initial_setup = block(move || -> Result<dto::InitialSetup, APIError> {
		let users = user_manager.list()?;
		let has_any_admin = users.iter().any(|u| u.is_admin());
		Ok(dto::InitialSetup {
			has_any_users: has_any_admin,
		})
	})
	.await?;
	Ok(Json(initial_setup))
}

#[put("/config")]
async fn apply_config(
	_admin_rights: AdminRights,
	config_manager: Data<config::Manager>,
	config: Json<dto::Config>,
) -> Result<HttpResponse, APIError> {
	block(move || config_manager.apply(&config.to_owned().into())).await?;
	Ok(HttpResponse::new(StatusCode::OK))
}

#[get("/settings")]
async fn get_settings(
	settings_manager: Data<settings::Manager>,
	_admin_rights: AdminRights,
) -> Result<Json<dto::Settings>, APIError> {
	let settings = block(move || settings_manager.read()).await?;
	Ok(Json(settings.into()))
}

#[put("/settings")]
async fn put_settings(
	_admin_rights: AdminRights,
	settings_manager: Data<settings::Manager>,
	new_settings: Json<dto::NewSettings>,
) -> Result<HttpResponse, APIError> {
	block(move || settings_manager.amend(&new_settings.to_owned().into())).await?;
	Ok(HttpResponse::new(StatusCode::OK))
}

#[get("/mount_dirs")]
async fn list_mount_dirs(
	vfs_manager: Data<vfs::Manager>,
	_admin_rights: AdminRights,
) -> Result<Json<Vec<dto::MountDir>>, APIError> {
	let mount_dirs = block(move || vfs_manager.mount_dirs()).await?;
	let mount_dirs = mount_dirs.into_iter().map(|m| m.into()).collect();
	Ok(Json(mount_dirs))
}

#[put("/mount_dirs")]
async fn put_mount_dirs(
	_admin_rights: AdminRights,
	vfs_manager: Data<vfs::Manager>,
	new_mount_dirs: Json<Vec<dto::MountDir>>,
) -> Result<HttpResponse, APIError> {
	let new_mount_dirs: Vec<MountDir> = new_mount_dirs.iter().cloned().map(|m| m.into()).collect();
	block(move || vfs_manager.set_mount_dirs(&new_mount_dirs)).await?;
	Ok(HttpResponse::new(StatusCode::OK))
}

#[get("/ddns")]
async fn get_ddns_config(
	ddns_manager: Data<ddns::Manager>,
	_admin_rights: AdminRights,
) -> Result<Json<dto::DDNSConfig>, APIError> {
	let ddns_config = block(move || ddns_manager.config()).await?;
	Ok(Json(ddns_config.into()))
}

#[put("/ddns")]
async fn put_ddns_config(
	_admin_rights: AdminRights,
	ddns_manager: Data<ddns::Manager>,
	new_ddns_config: Json<dto::DDNSConfig>,
) -> Result<HttpResponse, APIError> {
	block(move || ddns_manager.set_config(&new_ddns_config.to_owned().into())).await?;
	Ok(HttpResponse::new(StatusCode::OK))
}

#[get("/users")]
async fn list_users(
	user_manager: Data<user::Manager>,
	_admin_rights: AdminRights,
) -> Result<Json<Vec<dto::User>>, APIError> {
	let users = block(move || user_manager.list()).await?;
	let users = users.into_iter().map(|u| u.into()).collect();
	Ok(Json(users))
}

#[post("/user")]
async fn create_user(
	user_manager: Data<user::Manager>,
	_admin_rights: AdminRights,
	new_user: Json<dto::NewUser>,
) -> Result<HttpResponse, APIError> {
	let new_user = new_user.to_owned().into();
	block(move || user_manager.create(&new_user)).await?;
	Ok(HttpResponse::new(StatusCode::OK))
}

#[put("/user/{name}")]
async fn update_user(
	user_manager: Data<user::Manager>,
	admin_rights: AdminRights,
	name: web::Path<String>,
	user_update: Json<dto::UserUpdate>,
) -> Result<HttpResponse, APIError> {
	if let Some(auth) = &admin_rights.auth {
		if auth.username == name.as_str() && user_update.new_is_admin == Some(false) {
			return Err(APIError::OwnAdminPrivilegeRemoval);
		}
	}

	block(move || -> Result<(), APIError> {
		if let Some(password) = &user_update.new_password {
			user_manager.set_password(&name, password)?;
		}
		if let Some(is_admin) = &user_update.new_is_admin {
			user_manager.set_is_admin(&name, *is_admin)?;
		}
		Ok(())
	})
	.await?;
	Ok(HttpResponse::new(StatusCode::OK))
}

#[delete("/user/{name}")]
async fn delete_user(
	user_manager: Data<user::Manager>,
	admin_rights: AdminRights,
	name: web::Path<String>,
) -> Result<HttpResponse, APIError> {
	if let Some(auth) = &admin_rights.auth {
		if auth.username == name.as_str() {
			return Err(APIError::DeletingOwnAccount);
		}
	}
	block(move || user_manager.delete(&name)).await?;
	Ok(HttpResponse::new(StatusCode::OK))
}

#[get("/preferences")]
async fn get_preferences(
	user_manager: Data<user::Manager>,
	auth: Auth,
) -> Result<Json<user::Preferences>, APIError> {
	let preferences = block(move || user_manager.read_preferences(&auth.username)).await?;
	Ok(Json(preferences))
}

#[put("/preferences")]
async fn put_preferences(
	user_manager: Data<user::Manager>,
	auth: Auth,
	preferences: Json<user::Preferences>,
) -> Result<HttpResponse, APIError> {
	block(move || user_manager.write_preferences(&auth.username, &preferences)).await?;
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
	user_manager: Data<user::Manager>,
	credentials: Json<dto::Credentials>,
) -> Result<HttpResponse, APIError> {
	let username = credentials.username.clone();
	let (user::AuthToken(token), is_admin) =
		block(move || -> Result<(user::AuthToken, bool), APIError> {
			let auth_token = user_manager.login(&credentials.username, &credentials.password)?;
			let is_admin = user_manager.is_admin(&credentials.username)?;
			Ok((auth_token, is_admin))
		})
		.await?;
	let authorization = dto::Authorization {
		username: username.clone(),
		token,
		is_admin,
	};
	let response = HttpResponse::Ok().json(authorization);
	Ok(response)
}

#[get("/browse")]
async fn browse_root(
	index: Data<Index>,
	_auth: Auth,
) -> Result<Json<Vec<index::CollectionFile>>, APIError> {
	let result = block(move || index.browse(Path::new(""))).await?;
	Ok(Json(result))
}

#[get("/browse/{path:.*}")]
async fn browse(
	index: Data<Index>,
	_auth: Auth,
	path: web::Path<String>,
) -> Result<Json<Vec<index::CollectionFile>>, APIError> {
	let result = block(move || {
		let path = percent_decode_str(&path).decode_utf8_lossy();
		index.browse(Path::new(path.as_ref()))
	})
	.await?;
	Ok(Json(result))
}

#[get("/flatten")]
async fn flatten_root(index: Data<Index>, _auth: Auth) -> Result<Json<Vec<index::Song>>, APIError> {
	let songs = block(move || index.flatten(Path::new(""))).await?;
	Ok(Json(songs))
}

#[get("/flatten/{path:.*}")]
async fn flatten(
	index: Data<Index>,
	_auth: Auth,
	path: web::Path<String>,
) -> Result<Json<Vec<index::Song>>, APIError> {
	let songs = block(move || {
		let path = percent_decode_str(&path).decode_utf8_lossy();
		index.flatten(Path::new(path.as_ref()))
	})
	.await?;
	Ok(Json(songs))
}

#[get("/random")]
async fn random(index: Data<Index>, _auth: Auth) -> Result<Json<Vec<index::Directory>>, APIError> {
	let result = block(move || index.get_random_albums(20)).await?;
	Ok(Json(result))
}

#[get("/recent")]
async fn recent(index: Data<Index>, _auth: Auth) -> Result<Json<Vec<index::Directory>>, APIError> {
	let result = block(move || index.get_recent_albums(20)).await?;
	Ok(Json(result))
}

#[get("/search")]
async fn search_root(
	index: Data<Index>,
	_auth: Auth,
) -> Result<Json<Vec<index::CollectionFile>>, APIError> {
	let result = block(move || index.search("")).await?;
	Ok(Json(result))
}

#[get("/search/{query:.*}")]
async fn search(
	index: Data<Index>,
	_auth: Auth,
	query: web::Path<String>,
) -> Result<Json<Vec<index::CollectionFile>>, APIError> {
	let result = block(move || index.search(&query)).await?;
	Ok(Json(result))
}

#[get("/audio/{path:.*}")]
async fn get_audio(
	vfs_manager: Data<vfs::Manager>,
	_auth: Auth,
	path: web::Path<String>,
) -> Result<MediaFile, APIError> {
	let audio_path = block(move || {
		let vfs = vfs_manager.get_vfs()?;
		let path = percent_decode_str(&path).decode_utf8_lossy();
		vfs.virtual_to_real(Path::new(path.as_ref()))
	})
	.await?;

	let named_file = NamedFile::open(audio_path).map_err(|_| APIError::AudioFileIOError)?;
	Ok(MediaFile::new(named_file))
}

#[get("/thumbnail/{path:.*}")]
async fn get_thumbnail(
	vfs_manager: Data<vfs::Manager>,
	thumbnails_manager: Data<thumbnail::Manager>,
	_auth: Auth,
	path: web::Path<String>,
	options_input: web::Query<dto::ThumbnailOptions>,
) -> Result<MediaFile, APIError> {
	let options = thumbnail::Options::from(options_input.0);

	let thumbnail_path = block(move || -> Result<PathBuf, APIError> {
		let vfs = vfs_manager.get_vfs()?;
		let path = percent_decode_str(&path).decode_utf8_lossy();
		let image_path = vfs.virtual_to_real(Path::new(path.as_ref()))?;
		thumbnails_manager
			.get_thumbnail(&image_path, &options)
			.map_err(|e| e.into())
	})
	.await?;

	let named_file = NamedFile::open(thumbnail_path).map_err(|_| APIError::ThumbnailFileIOError)?;

	Ok(MediaFile::new(named_file))
}

#[get("/playlists")]
async fn list_playlists(
	playlist_manager: Data<playlist::Manager>,
	auth: Auth,
) -> Result<Json<Vec<dto::ListPlaylistsEntry>>, APIError> {
	let playlist_names = block(move || playlist_manager.list_playlists(&auth.username)).await?;
	let playlists: Vec<dto::ListPlaylistsEntry> = playlist_names
		.into_iter()
		.map(|p| dto::ListPlaylistsEntry { name: p })
		.collect();

	Ok(Json(playlists))
}

#[put("/playlist/{name}")]
async fn save_playlist(
	playlist_manager: Data<playlist::Manager>,
	auth: Auth,
	name: web::Path<String>,
	playlist: Json<dto::SavePlaylistInput>,
) -> Result<HttpResponse, APIError> {
	block(move || playlist_manager.save_playlist(&name, &auth.username, &playlist.tracks)).await?;
	Ok(HttpResponse::new(StatusCode::OK))
}

#[get("/playlist/{name}")]
async fn read_playlist(
	playlist_manager: Data<playlist::Manager>,
	auth: Auth,
	name: web::Path<String>,
) -> Result<Json<Vec<index::Song>>, APIError> {
	let songs = block(move || playlist_manager.read_playlist(&name, &auth.username)).await?;
	Ok(Json(songs))
}

#[delete("/playlist/{name}")]
async fn delete_playlist(
	playlist_manager: Data<playlist::Manager>,
	auth: Auth,
	name: web::Path<String>,
) -> Result<HttpResponse, APIError> {
	block(move || playlist_manager.delete_playlist(&name, &auth.username)).await?;
	Ok(HttpResponse::new(StatusCode::OK))
}

#[put("/lastfm/now_playing/{path:.*}")]
async fn lastfm_now_playing(
	lastfm_manager: Data<lastfm::Manager>,
	user_manager: Data<user::Manager>,
	auth: Auth,
	path: web::Path<String>,
) -> Result<HttpResponse, APIError> {
	block(move || -> Result<(), APIError> {
		if !user_manager.is_lastfm_linked(&auth.username) {
			return Err(APIError::LastFMAccountNotLinked);
		}
		let path = percent_decode_str(&path).decode_utf8_lossy();
		lastfm_manager.now_playing(&auth.username, Path::new(path.as_ref()))?;
		Ok(())
	})
	.await?;
	Ok(HttpResponse::new(StatusCode::OK))
}

#[post("/lastfm/scrobble/{path:.*}")]
async fn lastfm_scrobble(
	lastfm_manager: Data<lastfm::Manager>,
	user_manager: Data<user::Manager>,
	auth: Auth,
	path: web::Path<String>,
) -> Result<HttpResponse, APIError> {
	block(move || -> Result<(), APIError> {
		if !user_manager.is_lastfm_linked(&auth.username) {
			return Err(APIError::LastFMAccountNotLinked);
		}
		let path = percent_decode_str(&path).decode_utf8_lossy();
		lastfm_manager.scrobble(&auth.username, Path::new(path.as_ref()))?;
		Ok(())
	})
	.await?;
	Ok(HttpResponse::new(StatusCode::OK))
}

#[get("/lastfm/link_token")]
async fn lastfm_link_token(
	lastfm_manager: Data<lastfm::Manager>,
	auth: Auth,
) -> Result<Json<dto::LastFMLinkToken>, APIError> {
	let user::AuthToken(value) =
		block(move || lastfm_manager.generate_link_token(&auth.username)).await?;
	Ok(Json(dto::LastFMLinkToken { value }))
}

#[get("/lastfm/link")]
async fn lastfm_link(
	lastfm_manager: Data<lastfm::Manager>,
	user_manager: Data<user::Manager>,
	payload: web::Query<dto::LastFMLink>,
) -> Result<HttpResponse, APIError> {
	let popup_content_string = block(move || {
		let auth_token = user::AuthToken(payload.auth_token.clone());
		let authorization =
			user_manager.authenticate(&auth_token, user::AuthorizationScope::LastFMLink)?;
		let lastfm_token = &payload.token;
		lastfm_manager.link(&authorization.username, lastfm_token)?;

		// Percent decode
		let base64_content = percent_decode_str(&payload.content).decode_utf8_lossy();

		// Base64 decode
		let popup_content = BASE64_STANDARD_NO_PAD
			.decode(base64_content.as_bytes())
			.map_err(|_| APIError::LastFMLinkContentBase64DecodeError)?;

		// UTF-8 decode
		str::from_utf8(&popup_content)
			.map_err(|_| APIError::LastFMLinkContentEncodingError)
			.map(|s| s.to_owned())
	})
	.await?;

	Ok(HttpResponse::build(StatusCode::OK)
		.content_type("text/html; charset=utf-8")
		.body(popup_content_string))
}

#[delete("/lastfm/link")]
async fn lastfm_unlink(
	lastfm_manager: Data<lastfm::Manager>,
	auth: Auth,
) -> Result<HttpResponse, APIError> {
	block(move || lastfm_manager.unlink(&auth.username)).await?;
	Ok(HttpResponse::new(StatusCode::OK))
}
