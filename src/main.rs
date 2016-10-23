extern crate core;
extern crate getopts;
extern crate hyper;
extern crate id3;
extern crate image;
extern crate iron;
extern crate mount;
extern crate oven;
extern crate params;
extern crate regex;
extern crate rustc_serialize;
extern crate staticfile;
extern crate toml;
extern crate url;

#[cfg(windows)]
extern crate uuid;
#[cfg(windows)]
extern crate winapi;
#[cfg(windows)]
extern crate kernel32;
#[cfg(windows)]
extern crate shell32;
#[cfg(windows)]
extern crate user32;

use getopts::Options;
use iron::prelude::*;
use mount::Mount;
use staticfile::Static;
use std::path;
use std::sync::Arc;

mod api;
mod collection;
mod config;
mod ddns;
mod error;
mod ui;
mod utils;
mod thumbnails;
mod vfs;

const DEFAULT_CONFIG_FILE_NAME: &'static str = "polaris.toml";

fn main() {

    // Parse CLI options
    let args: Vec<String> = std::env::args().collect();
    let mut options = Options::new();
    options.optopt("c", "config", "set the configuration file", "FILE");
    let matches = match options.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };
    let config_file_name = matches.opt_str("c").unwrap_or(DEFAULT_CONFIG_FILE_NAME.to_owned());

    // Parse config
    println!("Reading configuration from {}", config_file_name);
    let config_file = path::Path::new(config_file_name.as_str());
    let config = config::Config::parse(&config_file).unwrap();

    // Start server
    println!("Starting up server");
    let mut api_chain;
    {
        let api_handler;
        {
            let mut collection = collection::Collection::new();
            collection.load_config(&config).unwrap();
            let collection = Arc::new(collection);
            api_handler = api::get_api_handler(collection);
        }
        api_chain = Chain::new(api_handler);

        let auth_secret = config.secret.to_owned();
        let cookie_middleware = oven::new(auth_secret.into_bytes());
        api_chain.link(cookie_middleware);
    }

    let mut mount = Mount::new();
    mount.mount("/api/", api_chain);
    mount.mount("/", Static::new(path::Path::new("web")));
    let mut server = Iron::new(mount).http(("0.0.0.0", 5050)).unwrap();

    // Start DDNS updates
    match config.ddns {
        Some(ref ddns_config) => {
            let ddns_config = ddns_config.clone();
            std::thread::spawn(|| {
                ddns::run(ddns_config);
            });
        },
        None => (),    
    };

    // Run UI
    ui::run();

    println!("Shutting down server");
    server.close().unwrap();
}
