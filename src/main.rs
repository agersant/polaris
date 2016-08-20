extern crate core;
extern crate iron;
extern crate mount;
extern crate rustc_serialize;
extern crate staticfile;
extern crate url;

use std::sync::Arc;
use std::sync::Mutex;

use iron::prelude::*;
use mount::Mount;
use staticfile::Static;

mod api;
mod collection;

fn main() {

    let collection = collection::Collection::new();
    let collection = Arc::new(Mutex::new(collection));

    let mut mount = Mount::new();
    let api_handler = api::get_api_handler( collection );
    mount.mount("/static/", Static::new("samplemusic/"))
        .mount("/api/", api_handler);

    Iron::new(mount).http("localhost:3000").unwrap();
}
