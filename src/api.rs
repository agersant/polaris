use std::fs;
use std::io;
use std::path::*;
use std::ops::Deref;
use std::sync::Arc;

use iron::prelude::*;
use iron::headers::{Authorization, Basic};
use iron::{AroundMiddleware, Handler, status};
use mount::Mount;
use params;
use secure_session::middleware::{SessionMiddleware, SessionConfig};
use secure_session::session::{SessionManager, ChaCha20Poly1305SessionManager};
use serde_json;
use typemap;
use url::percent_encoding::percent_decode;

use db::DB;
use errors::*;
use thumbnails::*;
use utils::*;

const CURRENT_MAJOR_VERSION: i32 = 2;
const CURRENT_MINOR_VERSION: i32 = 0;

#[derive(Serialize)]
struct Version {
	major: i32,
	minor: i32,
}

impl Version {
	fn new(major: i32, minor: i32) -> Version {
		Version {
			major: major,
			minor: minor,
		}
	}
}

#[derive(Deserialize, Serialize)]
struct Session {
	username: String,
}

struct SessionKey {}

impl typemap::Key for SessionKey {
	type Value = Session;
}

pub fn get_handler(db: Arc<DB>) -> Result<Chain> {
	let api_handler = get_endpoints(db.clone());
	let mut api_chain = Chain::new(api_handler);

	let auth_secret = db.deref().get_auth_secret()?;
	let session_manager = ChaCha20Poly1305SessionManager::<Session>::from_password(auth_secret.as_bytes());
	let session_config = SessionConfig::default();
	let session_middleware =
		SessionMiddleware::<Session,
		                    SessionKey,
		                    ChaCha20Poly1305SessionManager<Session>>::new(session_manager, session_config);
	api_chain.link_around(session_middleware);

	Ok(api_chain)
}

fn get_endpoints(db: Arc<DB>) -> Mount {
	let mut api_handler = Mount::new();

	{
		let db = db.clone();
		api_handler.mount("/version/", self::version);
		api_handler.mount("/auth/",
		                  move |request: &mut Request| self::auth(request, db.deref()));
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
			auth_api_mount.mount("/serve/", move |request: &mut Request| {
				self::serve(request, db.deref())
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

			// Auth via Authorization header
			if let Some(auth) = req.headers.get::<Authorization<Basic>>() {
				if let Some(ref password) = auth.password {
					auth_success = self.db
						.auth(auth.username.as_str(), password.as_str())?;
					req.extensions
						.insert::<SessionKey>(Session { username: auth.username.clone() });
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
	let current_version = Version::new(CURRENT_MAJOR_VERSION, CURRENT_MINOR_VERSION);
	match serde_json::to_string(&current_version) {
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
	if db.auth(username.as_str(), password.as_str())? {
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
	let browse_result = db.browse(&path)?;

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
	let flatten_result = db.flatten(&path)?;

	let result_json = serde_json::to_string(&flatten_result);
	let result_json = match result_json {
		Ok(j) => j,
		Err(e) => return Err(IronError::new(e, status::InternalServerError)),
	};

	Ok(Response::with((status::Ok, result_json)))
}

fn random(_: &mut Request, db: &DB) -> IronResult<Response> {
	let random_result = db.get_random_albums(20)?;
	let result_json = serde_json::to_string(&random_result);
	let result_json = match result_json {
		Ok(j) => j,
		Err(e) => return Err(IronError::new(e, status::InternalServerError)),
	};
	Ok(Response::with((status::Ok, result_json)))
}

fn recent(_: &mut Request, db: &DB) -> IronResult<Response> {
	let recent_result = db.get_recent_albums(20)?;
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

	let real_path = db.locate(virtual_path.as_path());
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
