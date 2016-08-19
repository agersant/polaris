use core::str::Utf8Error;
use std::path::PathBuf;
use std::ops::Deref;

use iron::prelude::*;
use iron::status;
use mount::Mount;
use rustc_serialize::json;
use url::percent_encoding::percent_decode;

use collection;
use collection::CollectionError;

impl From<CollectionError> for IronError {
    fn from(err: CollectionError) -> IronError {
        match err {
            CollectionError::Io(e) => IronError::new(e, status::NotFound),
            CollectionError::PathDecoding => IronError::new(err, status::InternalServerError)
        }
    }
}

pub fn get_api_handler() -> Mount {
    let mut mount = Mount::new();
    mount.mount("/browse/", self::browse)
        .mount("/flatten/", self::flatten);
    mount
}

fn path_from_request(request: &Request) -> Result<PathBuf, Utf8Error> {
    let path_string = request.url.path().join("/");
    let decoded_path = try!(percent_decode(path_string.as_bytes()).decode_utf8());
    Ok(PathBuf::from(decoded_path.deref()))
}

fn browse(request: &mut Request) -> IronResult<Response> {
    let path = path_from_request(request);
    if path.is_err() {
        return Ok(Response::with(status::BadRequest));
    }
    let path = path.unwrap();
    let browse_result = try!(collection::browse(&path));

    let result_json = json::encode(&browse_result);
    if result_json.is_err() {
        return Ok(Response::with(status::InternalServerError));
    }
    let result_json = result_json.unwrap();

    println!("{:?}", browse_result); // TMP
    Ok(Response::with((status::Ok, result_json)))
}

fn flatten(request: &mut Request) -> IronResult<Response> {
    let path = path_from_request(request);
    if path.is_err() {
        return Ok(Response::with((status::BadRequest)));
    }
    collection::flatten(&path.unwrap());
    Ok(Response::with((status::Ok, "TODO Flatten data here")))
}
