use std::path::PathBuf;

use axum::{
	extract::{DefaultBodyLimit, Path, Query, State},
	response::{Html, IntoResponse, Response},
	routing::{delete, get, post, put},
	Json, Router,
};
use axum_extra::headers::Range;
use axum_extra::TypedHeader;
use axum_range::{KnownSize, Ranged};
use base64::{prelude::BASE64_STANDARD_NO_PAD, Engine};
use percent_encoding::percent_decode_str;

use crate::{
	app::{collection, config, ddns, lastfm, playlist, settings, thumbnail, user, vfs, App},
	server::{
		dto, error::APIError, APIMajorVersion, API_ARRAY_SEPARATOR, API_MAJOR_VERSION,
		API_MINOR_VERSION,
	},
};

use super::auth::{AdminRights, Auth};

pub fn router() -> Router<App> {
	Router::new()
		.route("/version", get(get_version))
		.route("/initial_setup", get(get_initial_setup))
		.route("/config", put(put_config))
		.route("/settings", get(get_settings))
		.route("/settings", put(put_settings))
		.route("/mount_dirs", get(get_mount_dirs))
		.route("/mount_dirs", put(put_mount_dirs))
		.route("/ddns", get(get_ddns))
		.route("/ddns", put(put_ddns))
		.route("/auth", post(post_auth))
		.route("/user", post(post_user))
		.route("/user/:name", delete(delete_user))
		.route("/user/:name", put(put_user))
		.route("/users", get(get_users))
		.route("/preferences", get(get_preferences))
		.route("/preferences", put(put_preferences))
		.route("/trigger_index", post(post_trigger_index))
		.route("/browse", get(get_browse_root))
		.route("/browse/*path", get(get_browse))
		.route("/flatten", get(get_flatten_root))
		.route("/flatten/*path", get(get_flatten))
		.route("/artists/:artist", get(get_artist))
		.route("/artists/:artists/albums/:name", get(get_album))
		.route("/random", get(get_random))
		.route("/recent", get(get_recent))
		.route("/search", get(get_search_root))
		.route("/search/*query", get(get_search))
		.route("/playlists", get(get_playlists))
		.route("/playlist/:name", put(put_playlist))
		.route("/playlist/:name", get(get_playlist))
		.route("/playlist/:name", delete(delete_playlist))
		.route("/audio/*path", get(get_audio))
		.route("/thumbnail/*path", get(get_thumbnail))
		.route("/lastfm/now_playing/*path", put(put_lastfm_now_playing))
		.route("/lastfm/scrobble/*path", post(post_lastfm_scrobble))
		.route("/lastfm/link_token", get(get_lastfm_link_token))
		.route("/lastfm/link", get(get_lastfm_link))
		.route("/lastfm/link", delete(delete_lastfm_link))
		// TODO figure out NormalizePathLayer and remove this
		// See https://github.com/tokio-rs/axum/discussions/2833
		.route("/browse/", get(get_browse_root))
		.route("/flatten/", get(get_flatten_root))
		.route("/random/", get(get_random))
		.route("/recent/", get(get_recent))
		.route("/search/", get(get_search_root))
		.layer(DefaultBodyLimit::max(10 * 1024 * 1024)) // 10MB
}

async fn get_version() -> Json<dto::Version> {
	let current_version = dto::Version {
		major: API_MAJOR_VERSION,
		minor: API_MINOR_VERSION,
	};
	Json(current_version)
}

async fn get_initial_setup(
	State(user_manager): State<user::Manager>,
) -> Result<Json<dto::InitialSetup>, APIError> {
	let initial_setup = {
		let users = user_manager.list().await?;
		let has_any_admin = users.iter().any(|u| u.is_admin());
		dto::InitialSetup {
			has_any_users: has_any_admin,
		}
	};
	Ok(Json(initial_setup))
}

async fn put_config(
	_admin_rights: AdminRights,
	State(config_manager): State<config::Manager>,
	Json(config): Json<dto::Config>,
) -> Result<(), APIError> {
	config_manager.apply(&config.into()).await?;
	Ok(())
}

async fn get_settings(
	State(settings_manager): State<settings::Manager>,
	_admin_rights: AdminRights,
) -> Result<Json<dto::Settings>, APIError> {
	let settings = settings_manager.read().await?;
	Ok(Json(settings.into()))
}

async fn put_settings(
	_admin_rights: AdminRights,
	State(settings_manager): State<settings::Manager>,
	Json(new_settings): Json<dto::NewSettings>,
) -> Result<(), APIError> {
	settings_manager
		.amend(&new_settings.to_owned().into())
		.await?;
	Ok(())
}

async fn get_mount_dirs(
	_admin_rights: AdminRights,
	State(vfs_manager): State<vfs::Manager>,
) -> Result<Json<Vec<dto::MountDir>>, APIError> {
	let mount_dirs = vfs_manager.mount_dirs().await?;
	let mount_dirs = mount_dirs.into_iter().map(|m| m.into()).collect();
	Ok(Json(mount_dirs))
}

async fn put_mount_dirs(
	_admin_rights: AdminRights,
	State(vfs_manager): State<vfs::Manager>,
	new_mount_dirs: Json<Vec<dto::MountDir>>,
) -> Result<(), APIError> {
	let new_mount_dirs: Vec<vfs::MountDir> =
		new_mount_dirs.iter().cloned().map(|m| m.into()).collect();
	vfs_manager.set_mount_dirs(&new_mount_dirs).await?;
	Ok(())
}

async fn get_ddns(
	_admin_rights: AdminRights,
	State(ddns_manager): State<ddns::Manager>,
) -> Result<Json<dto::DDNSConfig>, APIError> {
	let ddns_config = ddns_manager.config().await?;
	Ok(Json(ddns_config.into()))
}

async fn put_ddns(
	_admin_rights: AdminRights,
	State(ddns_manager): State<ddns::Manager>,
	Json(new_ddns_config): Json<dto::DDNSConfig>,
) -> Result<(), APIError> {
	ddns_manager.set_config(&new_ddns_config.into()).await?;
	Ok(())
}

async fn post_auth(
	State(user_manager): State<user::Manager>,
	credentials: Json<dto::Credentials>,
) -> Result<Json<dto::Authorization>, APIError> {
	let username = credentials.username.clone();

	let user::AuthToken(token) = user_manager
		.login(&credentials.username, &credentials.password)
		.await?;
	let is_admin = user_manager.is_admin(&credentials.username).await?;

	let authorization = dto::Authorization {
		username: username.clone(),
		token,
		is_admin,
	};

	Ok(Json(authorization))
}

async fn get_users(
	_admin_rights: AdminRights,
	State(user_manager): State<user::Manager>,
) -> Result<Json<Vec<dto::User>>, APIError> {
	let users = user_manager.list().await?;
	let users = users.into_iter().map(|u| u.into()).collect();
	Ok(Json(users))
}

async fn post_user(
	_admin_rights: AdminRights,
	State(user_manager): State<user::Manager>,
	Json(new_user): Json<dto::NewUser>,
) -> Result<(), APIError> {
	user_manager.create(&new_user.into()).await?;
	Ok(())
}

async fn put_user(
	admin_rights: AdminRights,
	State(user_manager): State<user::Manager>,
	Path(name): Path<String>,
	user_update: Json<dto::UserUpdate>,
) -> Result<(), APIError> {
	if let Some(auth) = &admin_rights.get_auth() {
		if auth.get_username() == name.as_str() && user_update.new_is_admin == Some(false) {
			return Err(APIError::OwnAdminPrivilegeRemoval);
		}
	}

	if let Some(password) = &user_update.new_password {
		user_manager.set_password(&name, password).await?;
	}

	if let Some(is_admin) = &user_update.new_is_admin {
		user_manager.set_is_admin(&name, *is_admin).await?;
	}

	Ok(())
}

async fn delete_user(
	admin_rights: AdminRights,
	State(user_manager): State<user::Manager>,
	Path(name): Path<String>,
) -> Result<(), APIError> {
	if let Some(auth) = &admin_rights.get_auth() {
		if auth.get_username() == name.as_str() {
			return Err(APIError::DeletingOwnAccount);
		}
	}
	user_manager.delete(&name).await?;
	Ok(())
}

async fn get_preferences(
	auth: Auth,
	State(user_manager): State<user::Manager>,
) -> Result<Json<user::Preferences>, APIError> {
	let preferences = user_manager.read_preferences(auth.get_username()).await?;
	Ok(Json(preferences))
}

async fn put_preferences(
	auth: Auth,
	State(user_manager): State<user::Manager>,
	Json(preferences): Json<user::Preferences>,
) -> Result<(), APIError> {
	user_manager
		.write_preferences(auth.get_username(), &preferences)
		.await?;
	Ok(())
}

async fn post_trigger_index(
	_admin_rights: AdminRights,
	State(updater): State<collection::Updater>,
) -> Result<(), APIError> {
	updater.trigger_scan();
	Ok(())
}

fn collection_files_to_response(
	files: Vec<collection::File>,
	api_version: APIMajorVersion,
) -> Response {
	match api_version {
		APIMajorVersion::V7 => Json(
			files
				.into_iter()
				.map(|f| f.into())
				.collect::<Vec<dto::v7::CollectionFile>>(),
		)
		.into_response(),
		APIMajorVersion::V8 => Json(
			files
				.into_iter()
				.map(|f| f.into())
				.collect::<Vec<dto::BrowserEntry>>(),
		)
		.into_response(),
	}
}

fn songs_to_response(files: Vec<collection::Song>, api_version: APIMajorVersion) -> Response {
	match api_version {
		APIMajorVersion::V7 => Json(
			files
				.into_iter()
				.map(|f| f.into())
				.collect::<Vec<dto::v7::Song>>(),
		)
		.into_response(),
		APIMajorVersion::V8 => Json(
			files
				.into_iter()
				.map(|f| f.into())
				.collect::<Vec<dto::Song>>(),
		)
		.into_response(),
	}
}

fn albums_to_response(albums: Vec<collection::Album>, api_version: APIMajorVersion) -> Response {
	match api_version {
		APIMajorVersion::V7 => Json(
			albums
				.into_iter()
				.map(|f| f.into())
				.collect::<Vec<dto::v7::Directory>>(),
		)
		.into_response(),
		APIMajorVersion::V8 => Json(
			albums
				.into_iter()
				.map(|f| f.into())
				.collect::<Vec<dto::AlbumHeader>>(),
		)
		.into_response(),
	}
}

async fn get_browse_root(
	_auth: Auth,
	api_version: APIMajorVersion,
	State(index_manager): State<collection::IndexManager>,
) -> Response {
	let result = match index_manager.browse(PathBuf::new()).await {
		Ok(r) => r,
		Err(e) => return APIError::from(e).into_response(),
	};
	collection_files_to_response(result, api_version)
}

async fn get_browse(
	_auth: Auth,
	api_version: APIMajorVersion,
	State(index_manager): State<collection::IndexManager>,
	Path(path): Path<PathBuf>,
) -> Response {
	let result = match index_manager.browse(path).await {
		Ok(r) => r,
		Err(e) => return APIError::from(e).into_response(),
	};
	collection_files_to_response(result, api_version)
}

async fn get_flatten_root(
	_auth: Auth,
	api_version: APIMajorVersion,
	State(browser): State<collection::Browser>,
) -> Response {
	let songs = match browser.flatten(std::path::Path::new("")).await {
		Ok(s) => s,
		Err(e) => return APIError::from(e).into_response(),
	};
	songs_to_response(songs, api_version)
}

async fn get_flatten(
	_auth: Auth,
	api_version: APIMajorVersion,
	State(browser): State<collection::Browser>,
	Path(path): Path<String>,
) -> Response {
	let path = percent_decode_str(&path).decode_utf8_lossy();
	let songs = match browser.flatten(std::path::Path::new(path.as_ref())).await {
		Ok(s) => s,
		Err(e) => return APIError::from(e).into_response(),
	};
	songs_to_response(songs, api_version)
}

async fn get_artist(
	_auth: Auth,
	State(index_manager): State<collection::IndexManager>,
	Path(artist): Path<String>,
) -> Result<Json<dto::Artist>, APIError> {
	let artist_key = collection::ArtistKey {
		name: (!artist.is_empty()).then_some(artist),
	};
	Ok(Json(index_manager.get_artist(&artist_key).await?.into()))
}

async fn get_album(
	_auth: Auth,
	State(index_manager): State<collection::IndexManager>,
	Path((artists, name)): Path<(String, String)>,
) -> Result<Json<dto::Album>, APIError> {
	let album_key = collection::AlbumKey {
		artists: artists
			.split(API_ARRAY_SEPARATOR)
			.map(str::to_owned)
			.collect::<Vec<_>>(),
		name: (!name.is_empty()).then_some(name),
	};
	Ok(Json(index_manager.get_album(&album_key).await?.into()))
}

async fn get_random(
	_auth: Auth,
	api_version: APIMajorVersion,
	State(index_manager): State<collection::IndexManager>,
) -> Response {
	let albums = match index_manager.get_random_albums(20).await {
		Ok(d) => d,
		Err(e) => return APIError::from(e).into_response(),
	};
	albums_to_response(albums, api_version)
}

async fn get_recent(
	_auth: Auth,
	api_version: APIMajorVersion,
	State(index_manager): State<collection::IndexManager>,
) -> Response {
	let albums = match index_manager.get_recent_albums(20).await {
		Ok(d) => d,
		Err(e) => return APIError::from(e).into_response(),
	};
	albums_to_response(albums, api_version)
}

async fn get_search_root(
	_auth: Auth,
	api_version: APIMajorVersion,
	State(browser): State<collection::Browser>,
) -> Response {
	let files = match browser.search("").await {
		Ok(f) => f,
		Err(e) => return APIError::from(e).into_response(),
	};
	collection_files_to_response(files, api_version)
}

async fn get_search(
	_auth: Auth,
	api_version: APIMajorVersion,
	State(browser): State<collection::Browser>,
	Path(query): Path<String>,
) -> Response {
	let files = match browser.search(&query).await {
		Ok(f) => f,
		Err(e) => return APIError::from(e).into_response(),
	};
	collection_files_to_response(files, api_version)
}

async fn get_playlists(
	auth: Auth,
	State(playlist_manager): State<playlist::Manager>,
) -> Result<Json<Vec<dto::ListPlaylistsEntry>>, APIError> {
	let playlist_names = playlist_manager.list_playlists(auth.get_username()).await?;
	let playlists: Vec<dto::ListPlaylistsEntry> = playlist_names
		.into_iter()
		.map(|p| dto::ListPlaylistsEntry { name: p })
		.collect();

	Ok(Json(playlists))
}

async fn put_playlist(
	auth: Auth,
	State(playlist_manager): State<playlist::Manager>,
	Path(name): Path<String>,
	playlist: Json<dto::SavePlaylistInput>,
) -> Result<(), APIError> {
	playlist_manager
		.save_playlist(&name, auth.get_username(), &playlist.tracks)
		.await?;
	Ok(())
}

async fn get_playlist(
	auth: Auth,
	api_version: APIMajorVersion,
	State(playlist_manager): State<playlist::Manager>,
	Path(name): Path<String>,
) -> Response {
	let songs = match playlist_manager
		.read_playlist(&name, auth.get_username())
		.await
	{
		Ok(s) => s,
		Err(e) => return APIError::from(e).into_response(),
	};
	songs_to_response(songs, api_version)
}

async fn delete_playlist(
	auth: Auth,
	State(playlist_manager): State<playlist::Manager>,
	Path(name): Path<String>,
) -> Result<(), APIError> {
	playlist_manager
		.delete_playlist(&name, auth.get_username())
		.await?;
	Ok(())
}

async fn get_audio(
	_auth: Auth,
	State(vfs_manager): State<vfs::Manager>,
	Path(path): Path<String>,
	range: Option<TypedHeader<Range>>,
) -> Result<impl IntoResponse, APIError> {
	let vfs = vfs_manager.get_vfs().await?;
	let path = percent_decode_str(&path).decode_utf8_lossy();
	let audio_path = vfs.virtual_to_real(std::path::Path::new(path.as_ref()))?;

	let Ok(file) = tokio::fs::File::open(audio_path).await else {
		return Err(APIError::AudioFileIOError);
	};

	let Ok(body) = KnownSize::file(file).await else {
		return Err(APIError::AudioFileIOError);
	};

	let range = range.map(|TypedHeader(r)| r);
	Ok(Ranged::new(range, body))
}

async fn get_thumbnail(
	_auth: Auth,
	State(vfs_manager): State<vfs::Manager>,
	State(thumbnails_manager): State<thumbnail::Manager>,
	Path(path): Path<String>,
	Query(options_input): Query<dto::ThumbnailOptions>,
	range: Option<TypedHeader<Range>>,
) -> Result<impl IntoResponse, APIError> {
	let options = thumbnail::Options::from(options_input);
	let vfs = vfs_manager.get_vfs().await?;
	let path = percent_decode_str(&path).decode_utf8_lossy();
	let image_path = vfs.virtual_to_real(std::path::Path::new(path.as_ref()))?;
	let thumbnail_path = thumbnails_manager.get_thumbnail(&image_path, &options)?;

	let Ok(file) = tokio::fs::File::open(thumbnail_path).await else {
		return Err(APIError::ThumbnailFileIOError);
	};

	let Ok(body) = KnownSize::file(file).await else {
		return Err(APIError::ThumbnailFileIOError);
	};

	let range = range.map(|TypedHeader(r)| r);
	Ok(Ranged::new(range, body))
}

async fn put_lastfm_now_playing(
	auth: Auth,
	State(lastfm_manager): State<lastfm::Manager>,
	State(user_manager): State<user::Manager>,
	Path(path): Path<String>,
) -> Result<(), APIError> {
	if !user_manager.is_lastfm_linked(auth.get_username()).await {
		return Err(APIError::LastFMAccountNotLinked);
	}
	let path = percent_decode_str(&path).decode_utf8_lossy();
	lastfm_manager
		.now_playing(auth.get_username(), std::path::Path::new(path.as_ref()))
		.await?;
	Ok(())
}

async fn post_lastfm_scrobble(
	auth: Auth,
	State(lastfm_manager): State<lastfm::Manager>,
	State(user_manager): State<user::Manager>,
	Path(path): Path<String>,
) -> Result<(), APIError> {
	if !user_manager.is_lastfm_linked(auth.get_username()).await {
		return Err(APIError::LastFMAccountNotLinked);
	}
	let path = percent_decode_str(&path).decode_utf8_lossy();
	lastfm_manager
		.scrobble(auth.get_username(), std::path::Path::new(path.as_ref()))
		.await?;
	Ok(())
}

async fn get_lastfm_link_token(
	auth: Auth,
	State(lastfm_manager): State<lastfm::Manager>,
) -> Result<Json<dto::LastFMLinkToken>, APIError> {
	let user::AuthToken(value) = lastfm_manager.generate_link_token(auth.get_username())?;
	Ok(Json(dto::LastFMLinkToken { value }))
}

async fn get_lastfm_link(
	State(lastfm_manager): State<lastfm::Manager>,
	State(user_manager): State<user::Manager>,
	Query(payload): Query<dto::LastFMLink>,
) -> Result<Html<String>, APIError> {
	let auth_token = user::AuthToken(payload.auth_token.clone());
	let authorization = user_manager
		.authenticate(&auth_token, user::AuthorizationScope::LastFMLink)
		.await?;
	let lastfm_token = &payload.token;
	lastfm_manager
		.link(&authorization.username, lastfm_token)
		.await?;

	// Percent decode
	let base64_content = percent_decode_str(&payload.content).decode_utf8_lossy();

	// Base64 decode
	let popup_content = BASE64_STANDARD_NO_PAD
		.decode(base64_content.as_bytes())
		.map_err(|_| APIError::LastFMLinkContentBase64DecodeError)?;

	// UTF-8 decode
	let popup_content_string = std::str::from_utf8(&popup_content)
		.map_err(|_| APIError::LastFMLinkContentEncodingError)
		.map(|s| s.to_owned())?;

	Ok(Html(popup_content_string))
}

async fn delete_lastfm_link(
	auth: Auth,
	State(lastfm_manager): State<lastfm::Manager>,
) -> Result<(), APIError> {
	lastfm_manager.unlink(auth.get_username()).await?;
	Ok(())
}
