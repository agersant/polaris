use core::str::Utf8Error;
use core::ops::DerefMut;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::Mutex;

use iron::prelude::*;
use iron::headers::CookiePair;
use iron::{BeforeMiddleware, status};
use mount::Mount;
use oven::prelude::*;
use params;
use rustc_serialize::json;
use url::percent_encoding::percent_decode;

use collection::*;
use error::*;

impl From<PError> for IronError {
    fn from(err: PError) -> IronError {
        match err {
            PError::Io(e) => IronError::new(e, status::NotFound),
            PError::PathDecoding => IronError::new(err, status::InternalServerError),
            PError::ConflictingMount => IronError::new(err, status::BadRequest),
            PError::PathNotInVfs => IronError::new(err, status::NotFound),
            PError::CannotServeDirectory => IronError::new(err, status::BadRequest),
            PError::ConfigFileOpenError => IronError::new(err, status::InternalServerError),
            PError::ConfigFileReadError => IronError::new(err, status::InternalServerError),
            PError::ConfigFileParseError => IronError::new(err, status::InternalServerError),
            PError::ConfigMountDirsParseError => IronError::new(err, status::InternalServerError),
            PError::ConfigUsersParseError => IronError::new(err, status::InternalServerError),
            PError::ConfigAlbumArtPatternParseError => {
                IronError::new(err, status::InternalServerError)
            }
            PError::AlbumArtSearchError => IronError::new(err, status::InternalServerError),
            PError::ID3ParseError => IronError::new(err, status::InternalServerError),
            PError::Unauthorized => IronError::new(err, status::Unauthorized),
            PError::IncorrectCredentials => IronError::new(err, status::BadRequest),
        }
    }
}

pub fn get_api_handler(collection: Arc<Mutex<Collection>>) -> Mount {
    let mut api_handler = Mount::new();
    
    {
        let collection = collection.clone();
        api_handler.mount("/auth/", move |request: &mut Request| {
            let acquired_collection = collection.deref().lock().unwrap();
            self::auth(request, acquired_collection.deref())
        });
    }

    {
        let mut auth_api_mount = Mount::new();
        {
            let collection = collection.clone();
            auth_api_mount.mount("/browse/", move |request: &mut Request| {
                let mut acquired_collection = collection.deref().lock().unwrap();
                self::browse(request, acquired_collection.deref_mut())
            });
        }
        {
            let collection = collection.clone();
            auth_api_mount.mount("/flatten/", move |request: &mut Request| {
                let mut acquired_collection = collection.deref().lock().unwrap();
                self::flatten(request, acquired_collection.deref_mut())
            });
        }
        {
            let collection = collection.clone();
            auth_api_mount.mount("/serve/", move |request: &mut Request| {
                let mut acquired_collection = collection.deref().lock().unwrap();
                self::serve(request, acquired_collection.deref_mut())
            });
        }

        let mut auth_api_chain = Chain::new(auth_api_mount);
        auth_api_chain.link_before(AuthRequirement);

        api_handler.mount("/", auth_api_chain);
    }
    api_handler
}

fn path_from_request(request: &Request) -> Result<PathBuf, Utf8Error> {
    let path_string = request.url.path().join("/");
    let decoded_path = try!(percent_decode(path_string.as_bytes()).decode_utf8());
    Ok(PathBuf::from(decoded_path.deref()))
}

struct AuthRequirement;
impl BeforeMiddleware for AuthRequirement {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        let auth_cookie = req.get_cookie("username");
        if auth_cookie.is_some() {
            Ok(())
        } else {
            Err(IronError::new(PError::Unauthorized, status::Unauthorized))
        }
    }
}

fn auth(request: &mut Request, collection: &Collection) -> IronResult<Response> {
    let input = request.get_ref::<params::Params>().unwrap();
    let username = match input.find(&["username"]) {
        Some(&params::Value::String(ref username)) => username,
        _ => return Err(IronError::from(PError::IncorrectCredentials)),
    };
    let password = match input.find(&["password"]) {
        Some(&params::Value::String(ref password)) => password,
        _ => return Err(IronError::from(PError::IncorrectCredentials)),
    };
    if collection.auth(username.as_str(), password.as_str()) {
        let mut response = Response::with((status::Ok, ""));
        let mut username_cookie = CookiePair::new("username".to_string(), username.clone());
        username_cookie.path = Some("/".to_owned());
        response.set_cookie(username_cookie);
        Ok(response)
    } else {
        Err(IronError::from(PError::IncorrectCredentials))
    }
}

fn browse(request: &mut Request, collection: &mut Collection) -> IronResult<Response> {
    let path = path_from_request(request);
    let path = match path {
        Err(e) => return Err(IronError::new(e, status::BadRequest)),
        Ok(p) => p,
    };
    let browse_result = try!(collection.browse(&path));

    let result_json = json::encode(&browse_result);
    let result_json = match result_json {
        Ok(j) => j,
        Err(e) => return Err(IronError::new(e, status::InternalServerError)),
    };

    Ok(Response::with((status::Ok, result_json)))
}

fn flatten(request: &mut Request, collection: &mut Collection) -> IronResult<Response> {
    let path = path_from_request(request);
    let path = match path {
        Err(e) => return Err(IronError::new(e, status::BadRequest)),
        Ok(p) => p,
    };
    let flatten_result = try!(collection.flatten(&path));

    let result_json = json::encode(&flatten_result);
    let result_json = match result_json {
        Ok(j) => j,
        Err(e) => return Err(IronError::new(e, status::InternalServerError)),
    };

    Ok(Response::with((status::Ok, result_json)))
}

fn serve(request: &mut Request, collection: &mut Collection) -> IronResult<Response> {
    let virtual_path = path_from_request(request);
    let virtual_path = match virtual_path {
        Err(e) => return Err(IronError::new(e, status::BadRequest)),
        Ok(p) => p,
    };

    let real_path = collection.locate(virtual_path.as_path());
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
        return Err(IronError::new(PError::CannotServeDirectory, status::BadRequest));
    }

    Ok(Response::with((status::Ok, real_path)))
}
