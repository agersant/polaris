use diesel::prelude::*;
use iron::prelude::*;
use iron::headers::{Authorization, Basic};
use iron::{AroundMiddleware, Handler, status};
use mount::Mount;
use router::Router;
use params;
use secure_session::middleware::{SessionMiddleware, SessionConfig};
use secure_session::session::{SessionManager, ChaCha20Poly1305SessionManager};
use serde_json;
use std::fs;
use std::io;
use std::path::*;
use std::ops::Deref;
use std::sync::Arc;
use typemap;
use url::percent_encoding::percent_decode;

use config;
use config::MiscSettings;
use db::{ConnectionSource, DB};
use db::misc_settings;
use errors::*;
use thumbnails::*;
use index;
use user;
use utils::*;
use vfs::VFSSource;

const CURRENT_MAJOR_VERSION: i32 = 2;
const CURRENT_MINOR_VERSION: i32 = 1;


#[derive(Deserialize, Serialize)]
struct Session {
	username: String,
}

struct SessionKey {}

impl typemap::Key for SessionKey {
	type Value = Session;
}

fn get_auth_secret<T>(db: &T) -> Result<String>
	where T: ConnectionSource
{
	use self::misc_settings::dsl::*;
	let connection = db.get_connection();
	let connection = connection.lock().unwrap();
	let connection = connection.deref();
	let misc: MiscSettings = misc_settings.get_result(connection)?;
	Ok(misc.auth_secret.to_owned())
}

pub fn get_handler(db: Arc<DB>) -> Result<Chain> {
	let api_handler = get_endpoints(db.clone());
	let mut api_chain = Chain::new(api_handler);

	let auth_secret = get_auth_secret(db.deref())?;
	let session_manager =
		ChaCha20Poly1305SessionManager::<Session>::from_password(auth_secret.as_bytes());
	let session_config = SessionConfig::default();
	let session_middleware =
		SessionMiddleware::<Session,
		                    SessionKey,
		                    ChaCha20Poly1305SessionManager<Session>>::new(session_manager,
		                                                            session_config);
	api_chain.link_around(session_middleware);

	Ok(api_chain)
}

fn get_endpoints(db: Arc<DB>) -> Mount {
	let mut api_handler = Mount::new();

	{
		api_handler.mount("/version/", self::version);
		{
			let db = db.clone();
			api_handler.mount("/auth/",
			                  move |request: &mut Request| self::auth(request, db.deref()));
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
			auth_api_mount.mount("/browse/",
			                     move |request: &mut Request| self::browse(request, db.deref()));
		}
		{
			let db = db.clone();
			auth_api_mount.mount("/flatten/",
			                     move |request: &mut Request| self::flatten(request, db.deref()));
		}
		{
			let db = db.clone();
			auth_api_mount.mount("/random/",
			                     move |request: &mut Request| self::random(request, db.deref()));
		}
		{
			let db = db.clone();
			auth_api_mount.mount("/recent/",
			                     move |request: &mut Request| self::recent(request, db.deref()));
		}
		{
			let db = db.clone();
			auth_api_mount.mount("/serve/",
			                     move |request: &mut Request| self::serve(request, db.deref()));
		}
		{
			let mut settings_router = Router::new();
			let get_db = db.clone();
			let put_db = db.clone();
			settings_router.get("/",
			                    move |request: &mut Request| {
				                    self::get_config(request, get_db.deref())
				                   },
			                    "get_settings");
			settings_router.put("/",
			                    move |request: &mut Request| {
				                    self::put_config(request, put_db.deref())
				                   },
			                    "put_config");
			auth_api_mount.mount("/settings/", settings_router);
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
		             handler: handler,
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
			let mut auth_success = false;

			// Skip auth for first time setup
			if user::count(self.db.deref())? == 0 {
				auth_success = true;
			}

			// Auth via Authorization header
			if !auth_success {
				if let Some(auth) = req.headers.get::<Authorization<Basic>>() {
					if let Some(ref password) = auth.password {
						auth_success =
							user::auth(self.db.deref(), auth.username.as_str(), password.as_str())?;
						req.extensions
							.insert::<SessionKey>(Session { username: auth.username.clone() });
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
	if user::auth(db, username.as_str(), password.as_str())? {
		request
			.extensions
			.insert::<SessionKey>(Session { username: username.clone() });
		Ok(Response::with((status::Ok, "")))
	} else {
		Err(Error::from(ErrorKind::IncorrectCredentials).into())
	}
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
		return Ok(Response::with((status::Ok, real_path)));
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
