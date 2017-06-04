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
extern crate params;
extern crate reqwest;
extern crate regex;
extern crate secure_session;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate staticfile;
extern crate sqlite;
extern crate toml;
extern crate typemap;
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

#[cfg(unix)]
extern crate unix_daemonize;

#[cfg(unix)]
use unix_daemonize::{daemonize_redirect, ChdirMode};

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

#[cfg(unix)]
fn daemonize() -> Result<()> {
	let mut log_file = utils::get_data_root()?;
	log_file.push("polaris.log");
	match daemonize_redirect(Some(&log_file), Some(&log_file), ChdirMode::NoChdir) {
		Ok(_) => Ok(()),
		Err(_) => bail!(ErrorKind::DaemonError)
	}
}

fn run() -> Result<()> {
	
	#[cfg(unix)]
	daemonize()?;

	// Parse CLI options
	let args: Vec<String> = std::env::args().collect();
	let mut options = Options::new();
	options.optopt("c", "config", "set the configuration file", "FILE");
	options.optopt("w", "web", "set the path to web client files", "DIRECTORY");
	let matches = options.parse(&args[1..])?;

	// Parse config
	let config_file_name = matches.opt_str("c");
	let config_file_path = config_file_name.map(|n| Path::new(n.as_str()).to_path_buf());
	let config = config::Config::parse(config_file_path)?;

	// Init VFS
	let vfs = Arc::new(vfs::Vfs::new(config.vfs.clone()));

	// Init index
	println!("Starting up index");
	let index = Arc::new(index::Index::new(vfs.clone(), &config.index)?);
	let index_ref = index.clone();
	std::thread::spawn(move || index_ref.run());

	// Mount API
	println!("Mounting API");
	let mut mount = Mount::new();
	let mut collection = collection::Collection::new(vfs, index);
	collection.load_config(&config)?;
	let handler = api::get_handler(collection, &config.secret);
	mount.mount("/api/", handler);

	// Mount static files
	println!("Mounting static files");
	let web_dir_name = matches.opt_str("w");
	let mut default_web_dir = utils::get_data_root()?;
	default_web_dir.push("web");
	let web_dir_path = web_dir_name
		.map(|n| Path::new(n.as_str()).to_path_buf())
		.unwrap_or(default_web_dir);

	mount.mount("/", Static::new(web_dir_path));

	println!("Starting up server");
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
