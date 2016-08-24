extern crate core;
extern crate iron;
extern crate mount;
extern crate rustc_serialize;
extern crate url;

use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

use iron::prelude::*;
use mount::Mount;

mod api;
mod collection;
mod error;
mod vfs;

use api::*;
use collection::*;

fn main() {

    let mut collection = Collection::new();
    collection.mount("root", Path::new("samplemusic/"));
    let collection = Arc::new(Mutex::new(collection));

    let mut mount = Mount::new();
    let api_handler = get_api_handler(collection);
    mount.mount("/api/", api_handler);

    Iron::new(mount).http("localhost:3000").unwrap();
}
