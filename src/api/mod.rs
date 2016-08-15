use core::str::Utf8Error;
use std::path::PathBuf;

use iron::prelude::*;
use iron::status;
use mount::Mount;
use url::percent_encoding::percent_decode;

use collection::browse as collection_browse;
use collection::flatten as collection_flatten;

pub fn get_api_handler() -> Mount {
    let mut mount = Mount::new();
    mount.mount("/browse/", self::browse)
        .mount("/flatten/", self::flatten);
    mount
}

fn path_from_request(request: &Request) -> Result<PathBuf, Utf8Error> {
    let path_string = request.url.path().join("/");
    let decoded_path = percent_decode(path_string.as_bytes()).decode_utf8();
    decoded_path.map(|s| PathBuf::from(s.into_owned()))
}

fn browse(request: &mut Request) -> IronResult<Response> {
    let path = path_from_request(request);
    if path.is_err() {
        return Ok(Response::with((status::BadRequest)));
    }
    collection_browse(&path.unwrap());
    Ok(Response::with((status::Ok, "TODO browse data here")))
}

fn flatten(request: &mut Request) -> IronResult<Response> {
    let path = path_from_request(request);
    if path.is_err() {
        return Ok(Response::with((status::BadRequest)));
    }
    collection_flatten(&path.unwrap());
    Ok(Response::with((status::Ok, "TODO Flatten data here")))
}
