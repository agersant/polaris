extern crate core;
extern crate iron;
extern crate mount;
extern crate oven;
extern crate params;
extern crate regex;
extern crate id3;
extern crate rustc_serialize;
extern crate staticfile;
extern crate toml;
extern crate url;

use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

use iron::prelude::*;
use mount::Mount;
use staticfile::Static;

mod api;
mod collection;
mod error;
mod vfs;

use api::*;
use collection::*;

fn main() {


    let mut api_chain;
    {
        let api_handler;
        {
            let mut collection = Collection::new();
            collection.load_config(Path::new("Polaris.toml")).unwrap();
            let collection = Arc::new(Mutex::new(collection));
            api_handler = get_api_handler(collection);
        }
        api_chain = Chain::new(api_handler);
        
        let auth_secret = std::env::var("POLARIS_SECRET").expect("Environment variable POLARIS_SECRET must be set");
        let cookie_middleware = oven::new(auth_secret.into_bytes());
        api_chain.link(cookie_middleware);
    }

    let mut mount = Mount::new();
    mount.mount("/api/", api_chain);
    mount.mount("/", Static::new(Path::new("web")));

    Iron::new(mount).http("localhost:3000").unwrap();
}
