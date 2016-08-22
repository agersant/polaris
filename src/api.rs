use core::str::Utf8Error;
use core::ops::DerefMut;
use std::path::PathBuf;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::Mutex;

use iron::prelude::*;
use iron::status;
use mount::Mount;
use rustc_serialize::json;
use url::percent_encoding::percent_decode;

use collection::*;
use error::*;

impl From<CollectionError> for IronError {
    fn from(err: CollectionError) -> IronError {
        match err {
            CollectionError::Io(e) => IronError::new(e, status::NotFound),
            CollectionError::PathDecoding => IronError::new(err, status::InternalServerError),
            CollectionError::ConflictingMount => IronError::new(err, status::BadRequest),
            CollectionError::PathNotInVfs => IronError::new(err, status::NotFound),
        }
    }
}

pub fn get_api_handler(collection: Arc<Mutex<Collection>>) -> Mount {
    let mut mount = Mount::new();
    {
        let collection = collection.clone();
        mount.mount("/browse/", move |request: &mut Request| {
            let mut acquired_collection = collection.deref().lock().unwrap();
            self::browse(request, acquired_collection.deref_mut())
        });
    }
    {
        let collection = collection.clone();
        mount.mount("/flatten/", move |request: &mut Request| {
            let mut acquired_collection = collection.deref().lock().unwrap();
            self::flatten(request, acquired_collection.deref_mut())
        });
    }
    mount
}

fn path_from_request(request: &Request) -> Result<PathBuf, Utf8Error> {
    let path_string = request.url.path().join("/");
    let decoded_path = try!(percent_decode(path_string.as_bytes()).decode_utf8());
    Ok(PathBuf::from(decoded_path.deref()))
}

fn browse(request: &mut Request, collection: &mut Collection) -> IronResult<Response> {
    let path = path_from_request(request);
    if path.is_err() {
        return Ok(Response::with(status::BadRequest));
    }
    let path = path.unwrap();
    let browse_result = try!(collection.browse(&path));

    let result_json = json::encode(&browse_result);
    if result_json.is_err() {
        return Ok(Response::with(status::InternalServerError));
    }
    let result_json = result_json.unwrap();

    Ok(Response::with((status::Ok, result_json)))
}

fn flatten(request: &mut Request, collection: &mut Collection) -> IronResult<Response> {
    let path = path_from_request(request);
    if path.is_err() {
        return Ok(Response::with((status::BadRequest)));
    }
    let path = path.unwrap();
    let flatten_result = try!(collection.flatten(&path));

    let result_json = json::encode(&flatten_result);
    if result_json.is_err() {
        return Ok(Response::with(status::InternalServerError));
    }
    let result_json = result_json.unwrap();

    Ok(Response::with((status::Ok, result_json)))
}
