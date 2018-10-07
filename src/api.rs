use base64;
use diesel::prelude::*;
use iron::headers::{Authorization, Basic, Range};
use iron::mime::Mime;
use iron::prelude::*;
use iron::{status, AroundMiddleware, Handler};
use mount::Mount;
use params;
use router::Router;
use crypto::scrypt;
use secure_session::middleware::{SessionConfig, SessionMiddleware};
use secure_session::session::ChaCha20Poly1305SessionManager;
use serde_json;
use std::fs;
use std::io;
use std::ops::Deref;
use std::path::*;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use typemap;
use url::percent_encoding::percent_decode;

use config;
use db::misc_settings;
use db::{ConnectionSource, DB};
use errors::*;
use index;
use lastfm;
use playlist;
use serve;
use thumbnails::*;
use user;
use utils::*;
use vfs::VFSSource;

const CURRENT_MAJOR_VERSION: i32 = 2;
const CURRENT_MINOR_VERSION: i32 = 2;

#[derive(Deserialize, Serialize)]
struct Session {
	username: String,
}

struct SessionKey {}

impl typemap::Key for SessionKey {
	type Value = Session;
}

fn get_auth_secret<T>(db: &T) -> Result<[u8; 32]>
where
	T: ConnectionSource,
{
	use self::misc_settings::dsl::*;
	let connection = db.get_connection();
	let misc: config::MiscSettings = misc_settings.get_result(connection.deref())?;

	let params = scrypt::ScryptParams::new(12, 8, 1);
	let mut secret = [0; 32];
	scrypt::scrypt(misc.auth_secret.as_bytes(), b"polaris-salt-and-pepper-with-cheese", &params, &mut secret);
	Ok(secret)
}

pub fn get_handler(db: &Arc<DB>, index: &Arc<Mutex<Sender<index::Command>>>) -> Result<Chain> {
	let api_handler = get_endpoints(&db.clone(), &index);
	let mut api_chain = Chain::new(api_handler);

	let auth_secret = get_auth_secret(db.deref())?;
	let session_manager =
		ChaCha20Poly1305SessionManager::<Session>::from_key(auth_secret);
	let session_config = SessionConfig::default();
	let session_middleware = SessionMiddleware::<
		Session,
		SessionKey,
		ChaCha20Poly1305SessionManager<Session>,
	>::new(session_manager, session_config);
	api_chain.link_around(session_middleware);

	Ok(api_chain)
}

fn get_endpoints(db: &Arc<DB>, index_channel: &Arc<Mutex<Sender<index::Command>>>) -> Mount {
	let mut api_handler = Mount::new();

	{
		api_handler.mount("/version/", self::version);
		{
			let db = db.clone();
			api_handler.mount("/auth/", move |request: &mut Request| {
				self::auth(request, db.deref())
			});
		}
		{
			let db = db.clone();
			api_handler.mount("/initial_setup/", move |request: &mut Request| {
				self::initial_setup(request, db.deref())
			});
		}
	}

	{
		let mut auth_api_mount = Mount::new();
		{
			let db = db.clone();
			auth_api_mount.mount("/browse/", move |request: &mut Request| {
				self::browse(request, db.deref())
			});
		}
		{
			let db = db.clone();
			auth_api_mount.mount("/flatten/", move |request: &mut Request| {
				self::flatten(request, db.deref())
			});
		}
		{
			let db = db.clone();
			auth_api_mount.mount("/random/", move |request: &mut Request| {
				self::random(request, db.deref())
			});
		}
		{
			let db = db.clone();
			auth_api_mount.mount("/recent/", move |request: &mut Request| {
				self::recent(request, db.deref())
			});
		}
		{
			let db = db.clone();
			auth_api_mount.mount("/search/", move |request: &mut Request| {
				self::search(request, db.deref())
			});
		}
		{
			let db = db.clone();
			auth_api_mount.mount("/serve/", move |request: &mut Request| {
				self::serve(request, db.deref())
			});
		}
		{
			let mut preferences_router = Router::new();
			let get_db = db.clone();
			let put_db = db.clone();
			preferences_router.get(
				"/",
				move |request: &mut Request| self::get_preferences(request, get_db.deref()),
				"get_preferences",
			);
			preferences_router.put(
				"/",
				move |request: &mut Request| self::put_preferences(request, put_db.deref()),
				"put_preferences",
			);
			auth_api_mount.mount("/preferences/", preferences_router);
		}
		{
			let mut settings_router = Router::new();
			let get_db = db.clone();
			let put_db = db.clone();
			settings_router.get(
				"/",
				move |request: &mut Request| self::get_config(request, get_db.deref()),
				"get_config",
			);
			settings_router.put(
				"/",
				move |request: &mut Request| self::put_config(request, put_db.deref()),
				"put_config",
			);

			let mut settings_api_chain = Chain::new(settings_router);
			let admin_req = AdminRequirement { db: db.clone() };
			settings_api_chain.link_around(admin_req);

			auth_api_mount.mount("/settings/", settings_api_chain);
		}
		{
			let index_channel = index_channel.clone();
			let mut reindex_router = Router::new();
			reindex_router.post(
				"/",
				move |_: &mut Request| self::trigger_index(index_channel.deref()),
				"trigger_index",
			);

			let mut reindex_api_chain = Chain::new(reindex_router);
			let admin_req = AdminRequirement { db: db.clone() };
			reindex_api_chain.link_around(admin_req);

			auth_api_mount.mount("/trigger_index/", reindex_api_chain);
		}
		{
			let mut playlist_router = Router::new();
			let put_db = db.clone();
			let list_db = db.clone();
			let read_db = db.clone();
			let delete_db = db.clone();
			playlist_router.put(
				"/",
				move |request: &mut Request| self::save_playlist(request, put_db.deref()),
				"save_playlist",
			);

			playlist_router.get(
				"/list",
				move |request: &mut Request| self::list_playlists(request, list_db.deref()),
				"list_playlists",
			);

			playlist_router.get(
				"/read/:playlist_name",
				move |request: &mut Request| self::read_playlist(request, read_db.deref()),
				"read_playlist",
			);

			playlist_router.delete(
				"/:playlist_name",
				move |request: &mut Request| self::delete_playlist(request, delete_db.deref()),
				"delete_playlist",
			);

			auth_api_mount.mount("/playlist/", playlist_router);
		}
		{
			let db = db.clone();
			auth_api_mount.mount("/lastfm/auth/", move |request: &mut Request| {
				self::lastfm_auth(request, db.deref())
			});
		}
		{
			let db = db.clone();
			auth_api_mount.mount("/lastfm/now_playing/", move |request: &mut Request| {
				self::lastfm_now_playing(request, db.deref())
			});
		}
		{
			let db = db.clone();
			auth_api_mount.mount("/lastfm/scrobble/", move |request: &mut Request| {
				self::lastfm_scrobble(request, db.deref())
			});
		}

		let mut auth_api_chain = Chain::new(auth_api_mount);
		let auth = AuthRequirement { db: db.clone() };
		auth_api_chain.link_around(auth);

		api_handler.mount("/", auth_api_chain);
	}

	api_handler
}

fn path_from_request(request: &Request) -> Result<PathBuf> {
	let path_string = request
		.url
		.path()
		.join(&::std::path::MAIN_SEPARATOR.to_string());
	let decoded_path = percent_decode(path_string.as_bytes()).decode_utf8()?;
	Ok(PathBuf::from(decoded_path.deref()))
}

struct AuthRequirement {
	db: Arc<DB>,
}

impl AroundMiddleware for AuthRequirement {
	fn around(self, handler: Box<Handler>) -> Box<Handler> {
		Box::new(AuthHandler {
			db: self.db,
			handler,
		}) as Box<Handler>
	}
}

struct AuthHandler {
	handler: Box<Handler>,
	db: Arc<DB>,
}

impl Handler for AuthHandler {
	fn handle(&self, req: &mut Request) -> IronResult<Response> {
		{
			// Skip auth for first time setup
			let mut auth_success = user::count(self.db.deref())? == 0;

			// Auth via Authorization header
			if !auth_success {
				if let Some(auth) = req.headers.get::<Authorization<Basic>>() {
					if let Some(ref password) = auth.password {
						auth_success =
							user::auth(self.db.deref(), auth.username.as_str(), password.as_str())?;
						if auth_success {
							req.extensions.insert::<SessionKey>(Session {
								username: auth.username.clone(),
							});
						}
					}
				}
			}

			// Auth via Session
			if !auth_success {
				auth_success = req.extensions.get::<SessionKey>().is_some();
			}

			// Reject
			if !auth_success {
				return Err(Error::from(ErrorKind::AuthenticationRequired).into());
			}
		}

		self.handler.handle(req)
	}
}

struct AdminRequirement {
	db: Arc<DB>,
}

impl AroundMiddleware for AdminRequirement {
	fn around(self, handler: Box<Handler>) -> Box<Handler> {
		Box::new(AdminHandler {
			db: self.db,
			handler,
		}) as Box<Handler>
	}
}

struct AdminHandler {
	handler: Box<Handler>,
	db: Arc<DB>,
}

impl Handler for AdminHandler {
	fn handle(&self, req: &mut Request) -> IronResult<Response> {
		{
			// Skip auth for first time setup
			let mut auth_success = user::count(self.db.deref())? == 0;

			if !auth_success {
				match req.extensions.get::<SessionKey>() {
					Some(s) => auth_success = user::is_admin(self.db.deref(), &s.username)?,
					_ => return Err(Error::from(ErrorKind::AuthenticationRequired).into()),
				}
			}

			if !auth_success {
				return Err(Error::from(ErrorKind::AdminPrivilegeRequired).into());
			}
		}

		self.handler.handle(req)
	}
}

fn version(_: &mut Request) -> IronResult<Response> {
	#[derive(Serialize)]
	struct Version {
		major: i32,
		minor: i32,
	}

	let current_version = Version {
		major: CURRENT_MAJOR_VERSION,
		minor: CURRENT_MINOR_VERSION,
	};

	match serde_json::to_string(&current_version) {
		Ok(result_json) => Ok(Response::with((status::Ok, result_json))),
		Err(e) => Err(IronError::new(e, status::InternalServerError)),
	}
}

fn initial_setup(_: &mut Request, db: &DB) -> IronResult<Response> {
	#[derive(Serialize)]
	struct InitialSetup {
		has_any_users: bool,
	};

	let initial_setup = InitialSetup {
		has_any_users: user::count(db)? > 0,
	};

	match serde_json::to_string(&initial_setup) {
		Ok(result_json) => Ok(Response::with((status::Ok, result_json))),
		Err(e) => Err(IronError::new(e, status::InternalServerError)),
	}
}

fn auth(request: &mut Request, db: &DB) -> IronResult<Response> {
	let username;
	let password;
	{
		let input = request.get_ref::<params::Params>().unwrap();
		username = match input.find(&["username"]) {
			Some(&params::Value::String(ref username)) => username.clone(),
			_ => return Err(Error::from(ErrorKind::MissingUsername).into()),
		};
		password = match input.find(&["password"]) {
			Some(&params::Value::String(ref password)) => password.clone(),
			_ => return Err(Error::from(ErrorKind::MissingPassword).into()),
		};
	}

	if !user::auth(db, username.as_str(), password.as_str())? {
		return Err(Error::from(ErrorKind::IncorrectCredentials).into());
	}

	request.extensions.insert::<SessionKey>(Session {
		username: username.clone(),
	});

	#[derive(Serialize)]
	struct AuthOutput {
		admin: bool,
	}

	let auth_output = AuthOutput {
		admin: user::is_admin(db.deref(), &username)?,
	};
	let result_json = serde_json::to_string(&auth_output);
	let result_json = match result_json {
		Ok(j) => j,
		Err(e) => return Err(IronError::new(e, status::InternalServerError)),
	};

	Ok(Response::with((status::Ok, result_json)))
}

fn browse(request: &mut Request, db: &DB) -> IronResult<Response> {
	let path = path_from_request(request);
	let path = match path {
		Err(e) => return Err(IronError::new(e, status::BadRequest)),
		Ok(p) => p,
	};
	let browse_result = index::browse(db, &path)?;

	let result_json = serde_json::to_string(&browse_result);
	let result_json = match result_json {
		Ok(j) => j,
		Err(e) => return Err(IronError::new(e, status::InternalServerError)),
	};

	Ok(Response::with((status::Ok, result_json)))
}

fn flatten(request: &mut Request, db: &DB) -> IronResult<Response> {
	let path = path_from_request(request);
	let path = match path {
		Err(e) => return Err(IronError::new(e, status::BadRequest)),
		Ok(p) => p,
	};
	let flatten_result = index::flatten(db, &path)?;

	let result_json = serde_json::to_string(&flatten_result);
	let result_json = match result_json {
		Ok(j) => j,
		Err(e) => return Err(IronError::new(e, status::InternalServerError)),
	};

	Ok(Response::with((status::Ok, result_json)))
}

fn random(_: &mut Request, db: &DB) -> IronResult<Response> {
	let random_result = index::get_random_albums(db, 20)?;
	let result_json = serde_json::to_string(&random_result);
	let result_json = match result_json {
		Ok(j) => j,
		Err(e) => return Err(IronError::new(e, status::InternalServerError)),
	};
	Ok(Response::with((status::Ok, result_json)))
}

fn recent(_: &mut Request, db: &DB) -> IronResult<Response> {
	let recent_result = index::get_recent_albums(db, 20)?;
	let result_json = serde_json::to_string(&recent_result);
	let result_json = match result_json {
		Ok(j) => j,
		Err(e) => return Err(IronError::new(e, status::InternalServerError)),
	};
	Ok(Response::with((status::Ok, result_json)))
}

fn search(request: &mut Request, db: &DB) -> IronResult<Response> {
	let query = request
		.url
		.path()
		.join(&::std::path::MAIN_SEPARATOR.to_string());
	let query = match percent_decode(query.as_bytes()).decode_utf8() {
		Ok(s) => s,
		Err(_) => return Err(Error::from(ErrorKind::EncodingError).into()),
	};
	let search_result = index::search(db, &query)?;
	let result_json = serde_json::to_string(&search_result);
	let result_json = match result_json {
		Ok(j) => j,
		Err(e) => return Err(IronError::new(e, status::InternalServerError)),
	};
	Ok(Response::with((status::Ok, result_json)))
}

fn serve(request: &mut Request, db: &DB) -> IronResult<Response> {
	let virtual_path = path_from_request(request);
	let virtual_path = match virtual_path {
		Err(e) => return Err(IronError::new(e, status::BadRequest)),
		Ok(p) => p,
	};

	let vfs = db.get_vfs()?;
	let real_path = vfs.virtual_to_real(&virtual_path);
	let real_path = match real_path {
		Err(e) => return Err(IronError::new(e, status::NotFound)),
		Ok(p) => p,
	};

	let metadata = match fs::metadata(real_path.as_path()) {
		Ok(meta) => meta,
		Err(e) => {
			let status = match e.kind() {
				io::ErrorKind::NotFound => status::NotFound,
				io::ErrorKind::PermissionDenied => status::Forbidden,
				_ => status::InternalServerError,
			};
			return Err(IronError::new(e, status));
		}
	};

	if !metadata.is_file() {
		return Err(Error::from(ErrorKind::CannotServeDirectory).into());
	}

	if is_song(real_path.as_path()) {
		let range_header = request.headers.get::<Range>();
		return serve::deliver(&real_path, range_header);
	}

	if is_image(real_path.as_path()) {
		return art(request, real_path.as_path());
	}

	Err(Error::from(ErrorKind::UnsupportedFileType).into())
}

fn art(_: &mut Request, real_path: &Path) -> IronResult<Response> {
	let thumb = get_thumbnail(real_path, 400);
	match thumb {
		Ok(path) => Ok(Response::with((status::Ok, path))),
		Err(e) => Err(IronError::from(e)),
	}
}

fn get_config(_: &mut Request, db: &DB) -> IronResult<Response> {
	let c = config::read(db)?;
	let result_json = serde_json::to_string(&c);
	let result_json = match result_json {
		Ok(j) => j,
		Err(e) => return Err(IronError::new(e, status::InternalServerError)),
	};
	Ok(Response::with((status::Ok, result_json)))
}

fn put_config(request: &mut Request, db: &DB) -> IronResult<Response> {
	let input = request.get_ref::<params::Params>().unwrap();
	let config = match input.find(&["config"]) {
		Some(&params::Value::String(ref config)) => config,
		_ => return Err(Error::from(ErrorKind::MissingConfig).into()),
	};
	let config = config::parse_json(config)?;
	config::amend(db, &config)?;
	Ok(Response::with(status::Ok))
}

fn get_preferences(request: &mut Request, db: &DB) -> IronResult<Response> {
	let username = match request.extensions.get::<SessionKey>() {
		Some(s) => s.username.clone(),
		None => return Err(Error::from(ErrorKind::AuthenticationRequired).into()),
	};

	let preferences = config::read_preferences(db, &username)?;
	let result_json = serde_json::to_string(&preferences);
	let result_json = match result_json {
		Ok(j) => j,
		Err(e) => return Err(IronError::new(e, status::InternalServerError)),
	};
	Ok(Response::with((status::Ok, result_json)))
}

fn put_preferences(request: &mut Request, db: &DB) -> IronResult<Response> {
	let username = match request.extensions.get::<SessionKey>() {
		Some(s) => s.username.clone(),
		None => return Err(Error::from(ErrorKind::AuthenticationRequired).into()),
	};

	let input = request.get_ref::<params::Params>().unwrap();
	let preferences = match input.find(&["preferences"]) {
		Some(&params::Value::String(ref preferences)) => preferences,
		_ => return Err(Error::from(ErrorKind::MissingPreferences).into()),
	};
	let preferences = match serde_json::from_str::<config::Preferences>(preferences) {
		Ok(p) => p,
		Err(e) => return Err(IronError::new(e, status::InternalServerError)),
	};

	config::write_preferences(db, &username, &preferences)?;
	Ok(Response::with(status::Ok))
}

fn trigger_index(channel: &Mutex<Sender<index::Command>>) -> IronResult<Response> {
	let channel = channel.lock().unwrap();
	let channel = channel.deref();
	if let Err(e) = channel.send(index::Command::REINDEX) {
		return Err(IronError::new(e, status::InternalServerError));
	};
	Ok(Response::with(status::Ok))
}

fn save_playlist(request: &mut Request, db: &DB) -> IronResult<Response> {
	let username = match request.extensions.get::<SessionKey>() {
		Some(s) => s.username.clone(),
		None => return Err(Error::from(ErrorKind::AuthenticationRequired).into()),
	};

	let input = request.get_ref::<params::Params>().unwrap();
	let playlist = match input.find(&["playlist"]) {
		Some(&params::Value::String(ref playlist)) => playlist,
		_ => return Err(Error::from(ErrorKind::MissingPlaylist).into()),
	};

	#[derive(Deserialize)]
	struct SavePlaylistInput {
		name: String,
		tracks: Vec<String>,
	}

	let playlist = match serde_json::from_str::<SavePlaylistInput>(playlist) {
		Ok(p) => p,
		Err(e) => return Err(IronError::new(e, status::BadRequest)),
	};

	playlist::save_playlist(&playlist.name, &username, &playlist.tracks, db)?;

	Ok(Response::with(status::Ok))
}

fn list_playlists(request: &mut Request, db: &DB) -> IronResult<Response> {
	let username = match request.extensions.get::<SessionKey>() {
		Some(s) => s.username.clone(),
		None => return Err(Error::from(ErrorKind::AuthenticationRequired).into()),
	};

	#[derive(Serialize)]
	struct ListPlaylistsOutput {
		name: String,
	}

	let playlist_name = playlist::list_playlists(&username, db)?;
	let playlists: Vec<ListPlaylistsOutput> = playlist_name
		.into_iter()
		.map(|p| ListPlaylistsOutput { name: p })
		.collect();

	let result_json = serde_json::to_string(&playlists);
	let result_json = match result_json {
		Ok(j) => j,
		Err(e) => return Err(IronError::new(e, status::InternalServerError)),
	};
	Ok(Response::with((status::Ok, result_json)))
}

fn read_playlist(request: &mut Request, db: &DB) -> IronResult<Response> {
	let username = match request.extensions.get::<SessionKey>() {
		Some(s) => s.username.clone(),
		None => return Err(Error::from(ErrorKind::AuthenticationRequired).into()),
	};

	let params = request.extensions.get::<Router>().unwrap();
	let playlist_name = &(match params.find("playlist_name") {
		Some(s) => s,
		_ => return Err(Error::from(ErrorKind::MissingPlaylistName).into()),
	});

	let playlist_name = match percent_decode(playlist_name.as_bytes()).decode_utf8() {
		Ok(s) => s,
		Err(_) => return Err(Error::from(ErrorKind::EncodingError).into()),
	};

	let songs = playlist::read_playlist(&playlist_name, &username, db)?;
	let result_json = serde_json::to_string(&songs);
	let result_json = match result_json {
		Ok(j) => j,
		Err(e) => return Err(IronError::new(e, status::InternalServerError)),
	};

	Ok(Response::with((status::Ok, result_json)))
}

fn delete_playlist(request: &mut Request, db: &DB) -> IronResult<Response> {
	let username = match request.extensions.get::<SessionKey>() {
		Some(s) => s.username.clone(),
		None => return Err(Error::from(ErrorKind::AuthenticationRequired).into()),
	};

	let params = request.extensions.get::<Router>().unwrap();
	let playlist_name = &(match params.find("playlist_name") {
		Some(s) => s,
		_ => return Err(Error::from(ErrorKind::MissingPlaylistName).into()),
	});

	let playlist_name = match percent_decode(playlist_name.as_bytes()).decode_utf8() {
		Ok(s) => s,
		Err(_) => return Err(Error::from(ErrorKind::EncodingError).into()),
	};

	playlist::delete_playlist(&playlist_name, &username, db)?;

	Ok(Response::with(status::Ok))
}

fn lastfm_auth(request: &mut Request, db: &DB) -> IronResult<Response> {
	let input = request.get_ref::<params::Params>().unwrap();
	let username = match input.find(&["username"]) {
		Some(&params::Value::String(ref username)) => username.clone(),
		_ => return Err(Error::from(ErrorKind::MissingUsername).into()),
	};
	let token = match input.find(&["token"]) {
		Some(&params::Value::String(ref token)) => token.clone(),
		_ => return Err(Error::from(ErrorKind::MissingPassword).into()),
	};

	lastfm::auth(db, &username, &token)?;

	let url_encoded_content = match input.find(&["content"]) {
		Some(&params::Value::String(ref content)) => content.clone(),
		_ => return Err(Error::from(ErrorKind::MissingDesiredResponse).into()),
	};

	let base64_content = match percent_decode(url_encoded_content.as_bytes()).decode_utf8() {
		Ok(s) => s,
		Err(_) => return Err(Error::from(ErrorKind::EncodingError).into()),
	};

	let popup_content = match base64::decode(base64_content.as_bytes()) {
		Ok(c) => c,
		Err(_) => return Err(Error::from(ErrorKind::EncodingError).into()),
	};

	let mime = "text/html".parse::<Mime>().unwrap();

	Ok(Response::with((mime, status::Ok, popup_content)))
}

fn lastfm_now_playing(request: &mut Request, db: &DB) -> IronResult<Response> {
	let username = match request.extensions.get::<SessionKey>() {
		Some(s) => s.username.clone(),
		None => return Err(Error::from(ErrorKind::AuthenticationRequired).into()),
	};

	let virtual_path = path_from_request(request);
	let virtual_path = match virtual_path {
		Err(e) => return Err(IronError::new(e, status::BadRequest)),
		Ok(p) => p,
	};

	lastfm::now_playing(db, &username, &virtual_path)?;

	Ok(Response::with(status::Ok))
}

fn lastfm_scrobble(request: &mut Request, db: &DB) -> IronResult<Response> {
	let username = match request.extensions.get::<SessionKey>() {
		Some(s) => s.username.clone(),
		None => return Err(Error::from(ErrorKind::AuthenticationRequired).into()),
	};

	let virtual_path = path_from_request(request);
	let virtual_path = match virtual_path {
		Err(e) => return Err(IronError::new(e, status::BadRequest)),
		Ok(p) => p,
	};

	lastfm::scrobble(db, &username, &virtual_path)?;

	Ok(Response::with(status::Ok))
}
