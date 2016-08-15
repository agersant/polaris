extern crate core;
extern crate iron;
extern crate mount;
extern crate staticfile;
extern crate url;

use iron::prelude::*;
use mount::Mount;
use staticfile::Static;

mod api;
mod collection;

fn main() {
    let mut mount = Mount::new();
    let api_handler = api::get_api_handler();
    mount.mount("/static/", Static::new("samplemusic/"))
        .mount("/api/", api_handler);

    Iron::new(mount).http("localhost:3000").unwrap();
}
