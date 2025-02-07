use std::path::PathBuf;

use axum::{
	extract::{DefaultBodyLimit, Path, Query, State},
	response::{IntoResponse, Response},
	routing::get,
	Json,
};
use axum_extra::headers::Range;
use axum_extra::TypedHeader;
use axum_range::{KnownSize, Ranged};
use regex::Regex;
use tower_http::{compression::CompressionLayer, CompressionLevel};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
	app::{auth, config, ddns, index, peaks, playlist, scanner, thumbnail, App},
	server::{
		dto, error::APIError, APIMajorVersion, API_ARRAY_SEPARATOR, API_MAJOR_VERSION,
		API_MINOR_VERSION,
	},
};

use super::auth::{AdminRights, Auth};

pub fn router() -> OpenApiRouter<App> {
	OpenApiRouter::new()
		// Configuration
		.routes(routes!(get_version))
		.routes(routes!(get_initial_setup))
		.routes(routes!(get_settings, put_settings))
		.routes(routes!(get_mount_dirs, put_mount_dirs))
		.routes(routes!(post_trigger_index))
		.routes(routes!(get_index_status))
		// User management
		.routes(routes!(post_auth))
		.routes(routes!(post_user))
		.routes(routes!(delete_user, put_user))
		.routes(routes!(get_users))
		// File browser
		.routes(routes!(get_browse_root))
		.routes(routes!(get_browse))
		.routes(routes!(get_flatten_root))
		.routes(routes!(get_flatten))
		// Semantic
		.routes(routes!(get_albums))
		.routes(routes!(get_recent_albums))
		.routes(routes!(get_random_albums))
		.routes(routes!(get_artists))
		.routes(routes!(get_artist))
		.routes(routes!(get_album))
		.routes(routes!(get_genres))
		.routes(routes!(get_genre))
		.routes(routes!(get_genre_albums))
		.routes(routes!(get_genre_artists))
		.routes(routes!(get_genre_songs))
		.route("/random", get(get_random_albums)) // Deprecated
		.route("/recent", get(get_recent_albums)) // Deprecated
		// Search
		.routes(routes!(get_search))
		// Playlist management
		.routes(routes!(get_playlists))
		.routes(routes!(put_playlist, get_playlist, delete_playlist))
		// Media
		.routes(routes!(get_songs))
		.routes(routes!(get_peaks))
		.routes(routes!(get_thumbnail))
		// Layers
		.layer(CompressionLayer::new().quality(CompressionLevel::Fastest))
		.layer(DefaultBodyLimit::max(10 * 1024 * 1024)) // 10MB
		// Uncompressed
		.routes(routes!(get_audio))
}

#[utoipa::path(
	get,
	path = "/version",
	tag = "Configuration",
	description = "Returns the latest Polaris API version supported by this server.\n\nThe specification of this endpoint is guaranteed to remain consistent all API versions.",
	responses(
		(status = 200, body = dto::Version),
	),
)]
async fn get_version() -> Json<dto::Version> {
	let current_version = dto::Version {
		major: API_MAJOR_VERSION,
		minor: API_MINOR_VERSION,
	};
	Json(current_version)
}

#[utoipa::path(
	get,
	path = "/initial_setup",
	tag = "Configuration",
	description = "Returns the current state of the initial setup flow.",
	responses(
		(status = 200, body = dto::InitialSetup),
	),
)]
async fn get_initial_setup(
	State(config_manager): State<config::Manager>,
) -> Result<Json<dto::InitialSetup>, APIError> {
	let initial_setup = {
		let users = config_manager.get_users().await;
		let has_any_admin = users.iter().any(|u| u.admin == Some(true));
		dto::InitialSetup {
			has_any_users: has_any_admin,
		}
	};
	Ok(Json(initial_setup))
}

#[utoipa::path(
	get,
	path = "/settings",
	tag = "Configuration",
	description = "Reads current server settings.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	responses(
		(status = 200, body = dto::Settings),
	),
)]
async fn get_settings(
	_admin_rights: AdminRights,
	State(config_manager): State<config::Manager>,
) -> Result<Json<dto::Settings>, APIError> {
	let settings = dto::Settings {
		album_art_pattern: config_manager
			.get_index_album_art_pattern()
			.await
			.as_str()
			.to_owned(),
		ddns_update_url: config_manager
			.get_ddns_update_url()
			.await
			.as_ref()
			.map(http::Uri::to_string)
			.unwrap_or_default(),
	};
	Ok(Json(settings))
}

#[utoipa::path(
	put,
	path = "/settings",
	tag = "Configuration",
	description = "Amends the server settings. \n\n`null` fields are left unchanged.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	request_body = dto::NewSettings,
)]
async fn put_settings(
	_admin_rights: AdminRights,
	State(config_manager): State<config::Manager>,
	State(ddns_manager): State<ddns::Manager>,
	Json(new_settings): Json<dto::NewSettings>,
) -> Result<(), APIError> {
	if let Some(pattern) = new_settings.album_art_pattern {
		let Ok(regex) = Regex::new(&pattern) else {
			return Err(APIError::InvalidAlbumArtPattern);
		};
		config_manager.set_index_album_art_pattern(regex).await?;
	}

	if let Some(url_string) = new_settings.ddns_update_url {
		let uri = match url_string.trim() {
			"" => None,
			u => Some(http::Uri::try_from(u).or(Err(APIError::InvalidDDNSURL))?),
		};
		config_manager.set_ddns_update_url(uri).await?;
		ddns_manager.update_ddns().await?;
	}

	Ok(())
}

#[utoipa::path(
	get,
	path = "/mount_dirs",
	tag = "Configuration",
	description = "Returns the list of directories Polaris indexes music from.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	responses(
		(status = 200, body = Vec<dto::MountDir>),
	),
)]
async fn get_mount_dirs(
	_admin_rights: AdminRights,
	State(config_manager): State<config::Manager>,
) -> Result<Json<Vec<dto::MountDir>>, APIError> {
	let mount_dirs = config_manager.get_mounts().await;
	let mount_dirs = mount_dirs.into_iter().map(|m| m.into()).collect();
	Ok(Json(mount_dirs))
}

#[utoipa::path(
	put,
	path = "/mount_dirs",
	tag = "Configuration",
	description = "Replaces the list of directories Polaris indexes music from.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	request_body = Vec<dto::MountDir>,
)]
async fn put_mount_dirs(
	_admin_rights: AdminRights,
	State(config_manager): State<config::Manager>,
	new_mount_dirs: Json<Vec<dto::MountDir>>,
) -> Result<(), APIError> {
	let new_mount_dirs: Vec<config::storage::MountDir> =
		new_mount_dirs.iter().cloned().map(|m| m.into()).collect();
	config_manager.set_mounts(new_mount_dirs).await?;
	Ok(())
}

#[utoipa::path(
	post,
	path = "/auth",
	tag = "User Management",
	description = "Signs in a user. Tokens returned by this endpoint are required by most other endpoints.",
	responses(
		(status = 200, body = dto::Authorization),
		(status = 401),
	),
)]
async fn post_auth(
	State(config_manager): State<config::Manager>,
	credentials: Json<dto::Credentials>,
) -> Result<Json<dto::Authorization>, APIError> {
	let username = credentials.username.clone();

	let auth::Token(token) = config_manager
		.login(&credentials.username, &credentials.password)
		.await?;
	let user = config_manager.get_user(&credentials.username).await?;
	let is_admin = user.is_admin();

	let authorization = dto::Authorization {
		username: username.clone(),
		token,
		is_admin,
	};

	Ok(Json(authorization))
}

#[utoipa::path(
	get,
	path = "/users",
	tag = "User Management",
	description = "Lists existing user accounts.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	responses(
		(status = 200, body = Vec<dto::User>),
	),
)]
async fn get_users(
	_admin_rights: AdminRights,
	State(config_manager): State<config::Manager>,
) -> Result<Json<Vec<dto::User>>, APIError> {
	let users = config_manager.get_users().await;
	let users = users.into_iter().map(|u| u.into()).collect();
	Ok(Json(users))
}

#[utoipa::path(
	post,
	path = "/user",
	tag = "User Management",
	description = "Creates a new user account.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	request_body = dto::NewUser,
	responses(
		(status = 200),
		(status = 400),
		(status = 409)
	)
)]
async fn post_user(
	_admin_rights: AdminRights,
	State(config_manager): State<config::Manager>,
	Json(new_user): Json<dto::NewUser>,
) -> Result<(), APIError> {
	config_manager
		.create_user(&new_user.name, &new_user.password, new_user.admin)
		.await?;
	Ok(())
}

#[utoipa::path(
	put,
	path = "/user/{name}",
	tag = "User Management",
	description = "Amends an existing user account.\n\n`null` fields are left unchanged.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	request_body = dto::UserUpdate,
	responses(
		(status = 200),
		(status = 404),
		(status = 409)
	)
)]
async fn put_user(
	admin_rights: AdminRights,
	State(config_manager): State<config::Manager>,
	Path(name): Path<String>,
	user_update: Json<dto::UserUpdate>,
) -> Result<(), APIError> {
	if let Some(auth) = &admin_rights.get_auth() {
		if auth.get_username() == name.as_str() && user_update.new_is_admin == Some(false) {
			return Err(APIError::OwnAdminPrivilegeRemoval);
		}
	}

	if let Some(password) = &user_update.new_password {
		config_manager.set_password(&name, password).await?;
	}

	if let Some(is_admin) = &user_update.new_is_admin {
		config_manager.set_is_admin(&name, *is_admin).await?;
	}

	Ok(())
}

#[utoipa::path(
	delete,
	path = "/user/{name}",
	tag = "User Management",
	description = "Deletes a user account.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	responses(
		(status = 200),
		(status = 404),
		(status = 409)
	)
)]
async fn delete_user(
	admin_rights: AdminRights,
	State(config_manager): State<config::Manager>,
	Path(name): Path<String>,
) -> Result<(), APIError> {
	if let Some(auth) = &admin_rights.get_auth() {
		if auth.get_username() == name.as_str() {
			return Err(APIError::DeletingOwnAccount);
		}
	}
	config_manager.delete_user(&name).await?;
	Ok(())
}

#[utoipa::path(
	post,
	path = "/trigger_index",	
	tag = "Configuration",
	description = "Starts a scan of the mount directories that contain music files. If a scan is already in progress, it will be interrupted.\n\nThe music collection will update after the scan is fully completed.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
)]
async fn post_trigger_index(
	_admin_rights: AdminRights,
	State(scanner): State<scanner::Scanner>,
) -> Result<(), APIError> {
	scanner.try_trigger_scan();
	Ok(())
}

#[utoipa::path(
	get,
	path = "/index_status",
	tag = "Configuration",
	description = "Returns the current state of the collection scanning process.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	responses(
		(status = 200, body = dto::IndexStatus),
	)
)]
async fn get_index_status(
	_admin_rights: AdminRights,
	State(scanner): State<scanner::Scanner>,
) -> Result<Json<dto::IndexStatus>, APIError> {
	Ok(Json(scanner.get_status().await.into()))
}

fn index_files_to_response(files: Vec<index::File>, api_version: APIMajorVersion) -> Response {
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

const SONG_LIST_CAPACITY: usize = 200;

async fn make_song_list(paths: Vec<PathBuf>, index_manager: &index::Manager) -> dto::SongList {
	let first_paths = paths.iter().take(SONG_LIST_CAPACITY).cloned().collect();
	let first_songs = index_manager
		.get_songs(first_paths)
		.await
		.into_iter()
		.filter_map(Result::ok)
		.map(dto::Song::from)
		.collect();
	dto::SongList { paths, first_songs }
}

fn song_list_to_response(song_list: dto::SongList, api_version: APIMajorVersion) -> Response {
	match api_version {
		APIMajorVersion::V7 => Json(
			song_list
				.paths
				.into_iter()
				.map(|p| (&p).into())
				.collect::<Vec<dto::v7::Song>>(),
		)
		.into_response(),
		APIMajorVersion::V8 => Json(song_list).into_response(),
	}
}

fn albums_to_response(albums: Vec<index::Album>, api_version: APIMajorVersion) -> Response {
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
				.map(|f| f.header.into())
				.collect::<Vec<dto::AlbumHeader>>(),
		)
		.into_response(),
	}
}

#[utoipa::path(
	get,
	path = "/browse",
	tag = "File Browser",
	description = "Reads the content of the top-level directory in the music collection.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	params(
		("Accept-Version" = Option<i32>, Header, minimum = 7, maximum = 8)
	),
	responses(
		(status = 200, body = Vec<dto::BrowserEntry>),
	)
)]
async fn get_browse_root(
	_auth: Auth,
	api_version: APIMajorVersion,
	State(index_manager): State<index::Manager>,
) -> Response {
	let result = match index_manager.browse(PathBuf::new()).await {
		Ok(r) => r,
		Err(e) => return APIError::from(e).into_response(),
	};
	index_files_to_response(result, api_version)
}

#[utoipa::path(
	get,
	path = "/browse/{*path}",
	tag = "File Browser",
	description = "Reads the content of a directory in the music collection.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	params(
		("Accept-Version" = Option<i32>, Header, minimum = 7, maximum = 8),
		("path", allow_reserved, example = "my_music/classical/beethoven"),
	),
	responses(
		(status = 200, body = Vec<dto::BrowserEntry>),
	)
)]
async fn get_browse(
	_auth: Auth,
	api_version: APIMajorVersion,
	State(index_manager): State<index::Manager>,
	Path(path): Path<PathBuf>,
) -> Response {
	let result = match index_manager.browse(path).await {
		Ok(r) => r,
		Err(e) => return APIError::from(e).into_response(),
	};
	index_files_to_response(result, api_version)
}

#[utoipa::path(
	get,
	path = "/flatten",
	tag = "File Browser",
	description = "Recursively lists all the songs in the music collection.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	params(
		("Accept-Version" = Option<i32>, Header, minimum = 7, maximum = 8),
	),
	responses(
		(status = 200, body = dto::SongList),
	)
)]
async fn get_flatten_root(
	_auth: Auth,
	api_version: APIMajorVersion,
	State(index_manager): State<index::Manager>,
) -> Response {
	let paths = match index_manager.flatten(PathBuf::new()).await {
		Ok(s) => s,
		Err(e) => return APIError::from(e).into_response(),
	};
	let song_list = make_song_list(paths, &index_manager).await;
	song_list_to_response(song_list, api_version)
}

#[utoipa::path(
	get,
	path = "/flatten/{*path}",
	tag = "File Browser",
	description = "Recursively lists all the songs within a directory of the music collection.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	params(
		("Accept-Version" = Option<i32>, Header, minimum = 7, maximum = 8),
		("path", allow_reserved, example = "my_music/classical/beethoven"),
	),
	responses(
		(status = 200, body = dto::SongList),
	)
)]
async fn get_flatten(
	_auth: Auth,
	api_version: APIMajorVersion,
	State(index_manager): State<index::Manager>,
	Path(path): Path<PathBuf>,
) -> Response {
	let paths = match index_manager.flatten(path).await {
		Ok(s) => s,
		Err(e) => return APIError::from(e).into_response(),
	};
	let song_list = make_song_list(paths, &index_manager).await;
	song_list_to_response(song_list, api_version)
}

#[utoipa::path(
	get,
	path = "/albums",
	tag = "Collection",
	description = "Lists all albums in the music collection.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	responses(
		(status = 200, body = Vec<dto::AlbumHeader>),
	)
)]
async fn get_albums(
	_auth: Auth,
	State(index_manager): State<index::Manager>,
) -> Result<Json<Vec<dto::AlbumHeader>>, APIError> {
	Ok(Json(
		index_manager
			.get_albums()
			.await
			.into_iter()
			.map(|a| a.into())
			.collect::<Vec<_>>(),
	))
}

#[utoipa::path(
	get,
	path = "/artists",
	tag = "Collection",
	description = "Lists all artists in the music collection.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	responses(
		(status = 200, body = Vec<dto::ArtistHeader>),
	)
)]
async fn get_artists(
	_auth: Auth,
	State(index_manager): State<index::Manager>,
) -> Result<Json<Vec<dto::ArtistHeader>>, APIError> {
	Ok(Json(
		index_manager
			.get_artists()
			.await
			.into_iter()
			.map(|a| a.into())
			.collect::<Vec<_>>(),
	))
}

#[utoipa::path(
	get,
	path = "/artist/{name}",
	tag = "Collection",
	description = "Returns detailed information about a single artist.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	params(("name", example = "Claude Frank")),
	responses(
		(status = 200, body = dto::Artist),
	)
)]
async fn get_artist(
	_auth: Auth,
	State(index_manager): State<index::Manager>,
	Path(name): Path<String>,
) -> Result<Json<dto::Artist>, APIError> {
	Ok(Json(index_manager.get_artist(name).await?.into()))
}

#[utoipa::path(
	get,
	path = "/album/{name}/by/{artists}",
	tag = "Collection",
	description = "Returns detailed information about a single album.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	params(
		("name", example = "The Piano Sonatas"),
		("artists", example = "Claude Frank", description = "Artists the album is attributed to, separated by unicode \\u{000C} characters."),
	),
	responses(
		(status = 200, body = dto::Album),
	)
)]
async fn get_album(
	_auth: Auth,
	State(index_manager): State<index::Manager>,
	Path((name, artists)): Path<(String, String)>,
) -> Result<Json<dto::Album>, APIError> {
	let artists = artists
		.split(API_ARRAY_SEPARATOR)
		.map(str::to_owned)
		.collect::<Vec<_>>();
	Ok(Json(index_manager.get_album(artists, name).await?.into()))
}

#[utoipa::path(
	post, // post because of https://github.com/whatwg/fetch/issues/551
	path = "/songs",
	tag = "Collection",
	description = "Returns detailed information about specific songs.\n\nEven though it is a read operation, this endpoint uses the `POST` method in order to facilitate usage of a request body (which is not standard for `GET` requests).",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	request_body = dto::GetSongsBulkInput,
	responses(
		(status = 200, body = dto::GetSongsBulkOutput),
	)
)]
async fn get_songs(
	_auth: Auth,
	State(index_manager): State<index::Manager>,
	songs: Json<dto::GetSongsBulkInput>,
) -> Result<Json<dto::GetSongsBulkOutput>, APIError> {
	let results = index_manager
		.get_songs(songs.0.paths.clone())
		.await
		.into_iter()
		.collect::<Vec<_>>();

	let mut output = dto::GetSongsBulkOutput::default();
	for (i, r) in results.into_iter().enumerate() {
		match r {
			Ok(s) => output.songs.push(s.into()),
			Err(_) => output.not_found.push(songs.0.paths[i].clone()),
		}
	}

	Ok(Json(output))
}

#[utoipa::path(
	get,
	path = "/albums/random",
	tag = "Collection",
	description = "Returns a random selection of albums from the collection.\n\nRe-using the same seed will return the same albums only as long as the collection does not change.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	params(
		("Accept-Version" = Option<i32>, Header, minimum = 7, maximum = 8),
		dto::GetRandomAlbumsParameters,
	),
	responses(
		(status = 200, body = Vec<dto::AlbumHeader>),
	)
)]
async fn get_random_albums(
	_auth: Auth,
	api_version: APIMajorVersion,
	State(index_manager): State<index::Manager>,
	Query(options): Query<dto::GetRandomAlbumsParameters>,
) -> Response {
	let offset = options.offset.unwrap_or(0);
	let count = options.count.unwrap_or(20);
	let albums = match index_manager
		.get_random_albums(options.seed, offset, count)
		.await
	{
		Ok(d) => d,
		Err(e) => return APIError::from(e).into_response(),
	};
	albums_to_response(albums, api_version)
}

#[utoipa::path(
	get,
	path = "/albums/recent",
	tag = "Collection",
	description = "Returns the albums most recently added to the collection.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	params(
		("Accept-Version" = Option<i32>, Header, minimum = 7, maximum = 8),
		dto::GetRecentAlbumsParameters
	),
	responses(
		(status = 200, body = Vec<dto::AlbumHeader>),
	)
)]
async fn get_recent_albums(
	_auth: Auth,
	api_version: APIMajorVersion,
	State(index_manager): State<index::Manager>,
	Query(options): Query<dto::GetRecentAlbumsParameters>,
) -> Response {
	let offset = options.offset.unwrap_or(0);
	let count = options.count.unwrap_or(20);
	let albums = match index_manager.get_recent_albums(offset, count).await {
		Ok(d) => d,
		Err(e) => return APIError::from(e).into_response(),
	};
	albums_to_response(albums, api_version)
}

#[utoipa::path(
	get,
	path = "/genres",
	tag = "Collection",
	description = "Lists all music genres in the collection.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	responses(
		(status = 200, body = Vec<dto::GenreHeader>),
	)
)]
async fn get_genres(
	_auth: Auth,
	State(index_manager): State<index::Manager>,
) -> Result<Json<Vec<dto::GenreHeader>>, APIError> {
	Ok(Json(
		index_manager
			.get_genres()
			.await
			.into_iter()
			.map(|g| g.into())
			.collect(),
	))
}

#[utoipa::path(
	get,
	path = "/genre/{name}",
	tag = "Collection",
	description = "Returns detailed information about a music genre.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	params(("name", example = "Classical")),
	responses(
		(status = 200, body = Vec<dto::Genre>),
	)
)]
async fn get_genre(
	_auth: Auth,
	State(index_manager): State<index::Manager>,
	Path(name): Path<String>,
) -> Result<Json<dto::Genre>, APIError> {
	Ok(Json(index_manager.get_genre(name).await?.into()))
}

#[utoipa::path(
	get,
	path = "/genre/{name}/albums",
	tag = "Collection",
	description = "Returns all albums associated with a music genre.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	params(("name", example = "Classical")),
	responses(
		(status = 200, body = Vec<dto::AlbumHeader>),
	)
)]
async fn get_genre_albums(
	_auth: Auth,
	State(index_manager): State<index::Manager>,
	Path(name): Path<String>,
) -> Result<Json<Vec<dto::AlbumHeader>>, APIError> {
	let albums = index_manager
		.get_genre(name)
		.await?
		.albums
		.into_iter()
		.map(|a| a.into())
		.collect();
	Ok(Json(albums))
}

#[utoipa::path(
	get,
	path = "/genre/{name}/artists",
	tag = "Collection",
	description = "Returns all artists associated with a music genre.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	params(("name", example = "Classical")),
	responses(
		(status = 200, body = Vec<dto::ArtistHeader>),
	)
)]
async fn get_genre_artists(
	_auth: Auth,
	State(index_manager): State<index::Manager>,
	Path(name): Path<String>,
) -> Result<Json<Vec<dto::ArtistHeader>>, APIError> {
	let artists = index_manager
		.get_genre(name)
		.await?
		.artists
		.into_iter()
		.map(|a| a.into())
		.collect();
	Ok(Json(artists))
}

#[utoipa::path(
	get,
	path = "/genre/{name}/songs",
	tag = "Collection",
	description = "Returns all songs associated with a music genre.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	params(("name", example = "Classical")),
	responses(
		(status = 200, body = dto::SongList),
	)
)]
async fn get_genre_songs(
	_auth: Auth,
	State(index_manager): State<index::Manager>,
	Path(name): Path<String>,
) -> Result<Json<dto::SongList>, APIError> {
	let songs = index_manager.get_genre(name).await?.songs;
	let song_list = dto::SongList {
		paths: songs.iter().map(|s| s.virtual_path.clone()).collect(),
		first_songs: songs
			.into_iter()
			.take(SONG_LIST_CAPACITY)
			.map(|s| s.into())
			.collect(),
	};
	Ok(Json(song_list))
}

#[utoipa::path(
	get,
	path = "/search/{*query}",
	tag = "Collection",
	description = "Returns songs matching a search query. The query syntax is documented in the search section of the Polaris web UI.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	params(
		("Accept-Version" = Option<i32>, Header, minimum = 7, maximum = 8),
		("query", allow_reserved, example = "sonata && moonlight"),
	),
	responses(
		(status = 200, body = dto::SongList),
	)
)]
async fn get_search(
	_auth: Auth,
	api_version: APIMajorVersion,
	State(index_manager): State<index::Manager>,
	Path(query): Path<String>,
) -> Response {
	let songs = match index_manager.search(query).await {
		Ok(f) => f,
		Err(e) => return APIError::from(e).into_response(),
	};

	let song_list = dto::SongList {
		paths: songs.iter().map(|s| s.virtual_path.clone()).collect(),
		first_songs: songs
			.into_iter()
			.take(SONG_LIST_CAPACITY)
			.map(|s| s.into())
			.collect(),
	};

	match api_version {
		APIMajorVersion::V7 => Json(
			song_list
				.paths
				.iter()
				.map(|p| dto::v7::CollectionFile::Song(p.into()))
				.collect::<Vec<_>>(),
		)
		.into_response(),
		APIMajorVersion::V8 => Json(song_list).into_response(),
	}
}

#[utoipa::path(
	get,
	path = "/playlists",
	tag = "Playlists",
	description = "Lists playlists owned by the current user.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	responses(
		(status = 200, body = Vec<dto::PlaylistHeader>),
	)
)]
async fn get_playlists(
	auth: Auth,
	State(playlist_manager): State<playlist::Manager>,
) -> Result<Json<Vec<dto::PlaylistHeader>>, APIError> {
	let playlists = playlist_manager.list_playlists(auth.get_username()).await?;
	let playlists = playlists.into_iter().map(|p| p.into()).collect();

	Ok(Json(playlists))
}

#[utoipa::path(
	put,
	path = "/playlist/{name}",
	tag = "Playlists",
	description = "Creates or updates a playlist for the current user.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	params(("name", example = "Chill Jazz")),
	request_body = dto::SavePlaylistInput,
)]
async fn put_playlist(
	auth: Auth,
	State(playlist_manager): State<playlist::Manager>,
	State(index_manager): State<index::Manager>,
	Path(name): Path<String>,
	playlist: Json<dto::SavePlaylistInput>,
) -> Result<(), APIError> {
	let songs = index_manager
		.get_songs(playlist.tracks.clone())
		.await
		.into_iter()
		.filter_map(|s| s.ok())
		.collect();
	playlist_manager
		.save_playlist(&name, auth.get_username(), songs)
		.await?;
	Ok(())
}

#[utoipa::path(
	get,
	path = "/playlist/{name}",
	tag = "Playlists",
	description = "Retrieves a playlist owned by the current user.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	params(
		("Accept-Version" = Option<i32>, Header, minimum = 7, maximum = 8),
		("name", example = "Chill Jazz"),
	),
	responses(
		(status = 200, body = dto::Playlist),
	)
)]
async fn get_playlist(
	auth: Auth,
	api_version: APIMajorVersion,
	State(index_manager): State<index::Manager>,
	State(playlist_manager): State<playlist::Manager>,
	Path(name): Path<String>,
) -> Response {
	let playlist = match playlist_manager
		.read_playlist(&name, auth.get_username())
		.await
	{
		Ok(s) => s,
		Err(e) => return APIError::from(e).into_response(),
	};

	match api_version {
		APIMajorVersion::V7 => Json(playlist.songs).into_response(),
		APIMajorVersion::V8 => Json(dto::Playlist {
			header: playlist.header.into(),
			songs: make_song_list(playlist.songs, &index_manager).await,
		})
		.into_response(),
	}
}

#[utoipa::path(
	delete,
	path = "/playlist/{name}",
	tag = "Playlists",
	description = "Deletes a playlist owned by the current user.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	params(("name", example = "Chill Jazz")),
)]
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

#[utoipa::path(
	get,
	path = "/audio/{*path}",
	tag = "Media",
	description = "Serves a music file.\n\nThis endpoint supports HTTP range requests to facilitate streaming.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	params(("path", allow_reserved, example = "my_music/beethoven/moonlight_sonata.mp3")),
	responses(
		(status = 206, body = [u8]),
		(status = 200, body = [u8]),
	)
)]
async fn get_audio(
	_auth: Auth,
	State(config_manager): State<config::Manager>,
	Path(path): Path<PathBuf>,
	range: Option<TypedHeader<Range>>,
) -> Result<impl IntoResponse, APIError> {
	let audio_path = config_manager.resolve_virtual_path(&path).await?;

	let Ok(file) = tokio::fs::File::open(audio_path).await else {
		return Err(APIError::AudioFileIOError);
	};

	let Ok(body) = KnownSize::file(file).await else {
		return Err(APIError::AudioFileIOError);
	};

	let range = range.map(|TypedHeader(r)| r);
	Ok(Ranged::new(range, body))
}

#[utoipa::path(
	get,
	path = "/peaks/{*path}",
	tag = "Media",
	description = "Returns loudness values regularly sampled throughout the specified song.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	params(("path", allow_reserved, example = "my_music/beethoven/moonlight_sonata.mp3")),
	responses(
		(status = 200, body = [u8]),
	)
)]
async fn get_peaks(
	_auth: Auth,
	State(config_manager): State<config::Manager>,
	State(peaks_manager): State<peaks::Manager>,
	Path(path): Path<PathBuf>,
) -> Result<dto::Peaks, APIError> {
	let audio_path = config_manager.resolve_virtual_path(&path).await?;
	let peaks = peaks_manager.get_peaks(&audio_path).await?;
	Ok(peaks.interleaved)
}

#[utoipa::path(
	get,
	path = "/thumbnail/{*path}",
	tag = "Media",
	description = "Serves an image file. Valid paths can be obtained from the `.artwork` field of `Song`, `Album` and `AlbumHeader` models.\n\nThis endpoint supports HTTP range requests to facilitate streaming.",
	security(
		("auth_token" = []),
		("auth_query_param" = []),
	),
	params(
		("path", allow_reserved, example = "my_music/beethoven/sonatas.jpg"),
		dto::ThumbnailOptions
	),
	responses(
		(status = 206, body = [u8]),
		(status = 200, body = [u8]),
	)
)]
async fn get_thumbnail(
	_auth: Auth,
	State(config_manager): State<config::Manager>,
	State(thumbnails_manager): State<thumbnail::Manager>,
	Path(path): Path<PathBuf>,
	Query(options_input): Query<dto::ThumbnailOptions>,
	range: Option<TypedHeader<Range>>,
) -> Result<impl IntoResponse, APIError> {
	let options = thumbnail::Options::from(options_input);
	let image_path = config_manager.resolve_virtual_path(&path).await?;

	let thumbnail_path = thumbnails_manager
		.get_thumbnail(&image_path, &options)
		.await?;

	let Ok(file) = tokio::fs::File::open(thumbnail_path).await else {
		return Err(APIError::ThumbnailFileIOError);
	};

	let Ok(body) = KnownSize::file(file).await else {
		return Err(APIError::ThumbnailFileIOError);
	};

	let range = range.map(|TypedHeader(r)| r);
	Ok(Ranged::new(range, body))
}
