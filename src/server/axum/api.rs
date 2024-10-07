use std::path::PathBuf;

use axum::{
	extract::{DefaultBodyLimit, Path, Query, State},
	response::{IntoResponse, Response},
	routing::{delete, get, post, put},
	Json, Router,
};
use axum_extra::headers::Range;
use axum_extra::TypedHeader;
use axum_range::{KnownSize, Ranged};
use tower_http::{compression::CompressionLayer, CompressionLevel};

use crate::{
	app::{ddns, index, peaks, playlist, scanner, settings, thumbnail, user, vfs, App},
	server::{
		dto, error::APIError, APIMajorVersion, API_ARRAY_SEPARATOR, API_MAJOR_VERSION,
		API_MINOR_VERSION,
	},
};

use super::auth::{AdminRights, Auth};

pub fn router() -> Router<App> {
	Router::new()
		// Basic
		.route("/version", get(get_version))
		.route("/initial_setup", get(get_initial_setup))
		.route("/auth", post(post_auth))
		// Configuration
		.route("/settings", get(get_settings))
		.route("/settings", put(put_settings))
		.route("/mount_dirs", get(get_mount_dirs))
		.route("/mount_dirs", put(put_mount_dirs))
		.route("/ddns", get(get_ddns))
		.route("/ddns", put(put_ddns))
		.route("/trigger_index", post(post_trigger_index))
		// User management
		.route("/user", post(post_user))
		.route("/user/:name", delete(delete_user))
		.route("/user/:name", put(put_user))
		.route("/users", get(get_users))
		.route("/preferences", get(get_preferences))
		.route("/preferences", put(put_preferences))
		// File browser
		.route("/browse", get(get_browse_root))
		.route("/browse/*path", get(get_browse))
		.route("/flatten", get(get_flatten_root))
		.route("/flatten/*path", get(get_flatten))
		// Semantic
		.route("/albums", get(get_albums))
		.route("/albums/recent", get(get_recent_albums))
		.route("/albums/random", get(get_random_albums))
		.route("/artists", get(get_artists))
		.route("/artists/:artist", get(get_artist))
		.route("/artists/:artists/albums/:name", get(get_album))
		.route("/genres", get(get_genres))
		.route("/genres/:genre", get(get_genre))
		.route("/genres/:genre/albums", get(get_genre_albums))
		.route("/genres/:genre/artists", get(get_genre_artists))
		.route("/genres/:genre/songs", get(get_genre_songs))
		.route("/random", get(get_random_albums)) // Deprecated
		.route("/recent", get(get_recent_albums)) // Deprecated
		// Search
		.route("/search/*query", get(get_search))
		// Playlist management
		.route("/playlists", get(get_playlists))
		.route("/playlist/:name", put(put_playlist))
		.route("/playlist/:name", get(get_playlist))
		.route("/playlist/:name", delete(delete_playlist))
		// Media
		.route("/songs", post(get_songs)) // post because of https://github.com/whatwg/fetch/issues/551
		.route("/peaks/*path", get(get_peaks))
		.route("/thumbnail/*path", get(get_thumbnail))
		// Layers
		.layer(CompressionLayer::new().quality(CompressionLevel::Fastest))
		.layer(DefaultBodyLimit::max(10 * 1024 * 1024)) // 10MB
		// Uncompressed
		.route("/audio/*path", get(get_audio))
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
	State(scanner): State<scanner::Scanner>,
) -> Result<(), APIError> {
	scanner.trigger_scan();
	Ok(())
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
			.collect::<Vec<_>>()
			.into(),
	))
}

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
			.collect::<Vec<_>>()
			.into(),
	))
}

async fn get_artist(
	_auth: Auth,
	State(index_manager): State<index::Manager>,
	Path(artist): Path<String>,
) -> Result<Json<dto::Artist>, APIError> {
	Ok(Json(index_manager.get_artist(artist).await?.into()))
}

async fn get_album(
	_auth: Auth,
	State(index_manager): State<index::Manager>,
	Path((artists, name)): Path<(String, String)>,
) -> Result<Json<dto::Album>, APIError> {
	let artists = artists
		.split(API_ARRAY_SEPARATOR)
		.map(str::to_owned)
		.collect::<Vec<_>>();
	Ok(Json(index_manager.get_album(artists, name).await?.into()))
}

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

async fn get_peaks(
	_auth: Auth,
	State(vfs_manager): State<vfs::Manager>,
	State(peaks_manager): State<peaks::Manager>,
	Path(path): Path<PathBuf>,
) -> Result<dto::Peaks, APIError> {
	let vfs = vfs_manager.get_vfs().await?;
	let audio_path = vfs.virtual_to_real(&path)?;
	let peaks = peaks_manager.get_peaks(&audio_path).await?;
	Ok(peaks.interleaved)
}

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

async fn get_genre(
	_auth: Auth,
	State(index_manager): State<index::Manager>,
	Path(genre): Path<String>,
) -> Result<Json<dto::Genre>, APIError> {
	Ok(Json(index_manager.get_genre(genre).await?.into()))
}

async fn get_genre_albums(
	_auth: Auth,
	State(index_manager): State<index::Manager>,
	Path(genre): Path<String>,
) -> Result<Json<Vec<dto::AlbumHeader>>, APIError> {
	let albums = index_manager
		.get_genre(genre)
		.await?
		.albums
		.into_iter()
		.map(|a| a.into())
		.collect();
	Ok(Json(albums))
}

async fn get_genre_artists(
	_auth: Auth,
	State(index_manager): State<index::Manager>,
	Path(genre): Path<String>,
) -> Result<Json<Vec<dto::ArtistHeader>>, APIError> {
	let artists = index_manager
		.get_genre(genre)
		.await?
		.artists
		.into_iter()
		.map(|a| a.into())
		.collect();
	Ok(Json(artists))
}

async fn get_genre_songs(
	_auth: Auth,
	State(index_manager): State<index::Manager>,
	Path(genre): Path<String>,
) -> Result<Json<dto::SongList>, APIError> {
	let songs = index_manager.get_genre(genre).await?.songs;
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

async fn get_playlists(
	auth: Auth,
	State(playlist_manager): State<playlist::Manager>,
) -> Result<Json<Vec<dto::PlaylistHeader>>, APIError> {
	let playlists = playlist_manager.list_playlists(auth.get_username()).await?;
	let playlists = playlists.into_iter().map(|p| p.into()).collect();

	Ok(Json(playlists))
}

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
	Path(path): Path<PathBuf>,
	range: Option<TypedHeader<Range>>,
) -> Result<impl IntoResponse, APIError> {
	let vfs = vfs_manager.get_vfs().await?;
	let audio_path = vfs.virtual_to_real(&path)?;

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
	Path(path): Path<PathBuf>,
	Query(options_input): Query<dto::ThumbnailOptions>,
	range: Option<TypedHeader<Range>>,
) -> Result<impl IntoResponse, APIError> {
	let options = thumbnail::Options::from(options_input);
	let vfs = vfs_manager.get_vfs().await?;
	let image_path = vfs.virtual_to_real(&path)?;

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
