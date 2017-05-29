#![recursion_limit = "128"]

extern crate ape;
extern crate app_dirs;
extern crate core;
#[macro_use]
extern crate error_chain;
extern crate getopts;
extern crate hyper;
extern crate id3;
extern crate image;
extern crate iron;
extern crate lewton;
extern crate metaflac;
extern crate mount;
extern crate ogg;
extern crate oven;
extern crate params;
extern crate regex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate staticfile;
extern crate sqlite;
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

use errors::*;
use getopts::Options;
use iron::prelude::*;
use mount::Mount;
use staticfile::Static;
use std::path::Path;
use std::sync::Arc;

mod api;
mod collection;
mod config;
mod ddns;
mod errors;
mod index;
mod metadata;
mod ui;
mod utils;
mod thumbnails;
mod vfs;

fn main() {
	if let Err(ref e) = run() {
		println!("Error: {}", e);

		for e in e.iter().skip(1) {
			println!("caused by: {}", e);
		}
		if let Some(backtrace) = e.backtrace() {
			println!("backtrace: {:?}", backtrace);
		}
		::std::process::exit(1);
	}
}

fn run() -> Result<()> {

	// Parse CLI options
	let args: Vec<String> = std::env::args().collect();
	let mut options = Options::new();
	options.optopt("c", "config", "set the configuration file", "FILE");
	let matches = match options.parse(&args[1..]) {
		Ok(m) => m,
		Err(f) => panic!(f.to_string()),
	};
	let config_file_name = matches.opt_str("c");
	let config_file_path = config_file_name.map(|n| Path::new(n.as_str()).to_path_buf());

	// Parse config
	let config = config::Config::parse(config_file_path)?;

	// Init VFS
	let vfs = Arc::new(vfs::Vfs::new(config.vfs.clone()));

	// Init index
	println!("Starting up index");
	let index = Arc::new(index::Index::new(vfs.clone(), &config.index)?);
	let index_ref = index.clone();
	std::thread::spawn(move || index_ref.run());

	// Start server
	println!("Starting up server");
	let mut api_chain;
	{
		let api_handler;
		{
			let mut collection = collection::Collection::new(vfs, index);
			collection.load_config(&config)?;
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
	mount.mount("/", Static::new(Path::new("web")));
	let mut server = Iron::new(mount).http(("0.0.0.0", 5050))?;

	// Start DDNS updates
	match config.ddns {
		Some(ref ddns_config) => {
			let ddns_config = ddns_config.clone();
			std::thread::spawn(|| { ddns::run(ddns_config); });
		}
		None => (),
	};

	// Run UI
	ui::run();

	println!("Shutting down server");
	server.close()?;

	Ok(())
}
