#![recursion_limit = "128"]

extern crate ape;
extern crate app_dirs;
extern crate core;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_infer_schema;
#[macro_use]
extern crate diesel_migrations;
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
extern crate mp3_duration;
extern crate params;
extern crate rand;
extern crate reqwest;
extern crate regex;
extern crate ring;
extern crate router;
extern crate secure_session;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate staticfile;
extern crate toml;
extern crate typemap;
extern crate url;
#[macro_use]
extern crate log;
extern crate simplelog;

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
#[cfg(unix)]
use std::fs::File;
#[cfg(unix)]
use std::io::prelude::*;

use core::ops::Deref;
use errors::*;
use getopts::Options;
use iron::prelude::*;
use mount::Mount;
use staticfile::Static;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::channel;
use simplelog::{Config, TermLogger, LogLevelFilter};
#[cfg(unix)]
use simplelog::{SimpleLogger};

mod api;
mod config;
mod db;
mod ddns;
mod errors;
mod index;
mod metadata;
mod playlist;
mod ui;
mod user;
mod utils;
mod serve;
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
fn daemonize(options: &getopts::Matches) -> Result<()> {
	if options.opt_present("f") {
		return Ok(());
	}
	let mut log_file = utils::get_data_root()?;
	log_file.push("polaris.log");
	let pid = match daemonize_redirect(Some(&log_file), Some(&log_file), ChdirMode::NoChdir) {
		Ok(p) => p,
		Err(_) => bail!(ErrorKind::DaemonError),
	};
	let mut pid_path = utils::get_data_root()?;
	pid_path.push("polaris.pid");
	let mut file = File::create(pid_path)?;
	file.write_all(pid.to_string().as_bytes())?;
	Ok(())
}

#[cfg(unix)]
fn init_log(log_level: LogLevelFilter, options: &getopts::Matches) -> Result<()> {
	if options.opt_present("f") {
		if let Err(e) = TermLogger::init(log_level, Config::default()) {
			bail!("Error starting terminal logger: {}", e);
		};
	}
	else
	{
		if let Err(e) = SimpleLogger::init(log_level, Config::default()) {
			bail!("Error starting simple logger: {}", e);
		}
	}
	Ok(())
}

#[cfg(windows)]
fn init_log(log_level: LogLevelFilter, _: &getopts::Matches) -> Result<()> {
	if let Err(e) = TermLogger::init(log_level, Config::default()) {
		bail!("Error starting terminal logger: {}", e);
	};
	Ok(())
}

fn run() -> Result<()> {

	// Parse CLI options
	let args: Vec<String> = std::env::args().collect();
	let mut options = Options::new();
	options.optopt("c", "config", "set the configuration file", "FILE");
	options.optopt("p", "port", "set polaris to run on a custom port", "PORT");
	options.optopt("d", "database", "set the path to index database", "FILE");
	options.optopt("w", "web", "set the path to web client files", "DIRECTORY");
	options.optopt("l",
	               "log",
	               "set the log level to a value between 0 (off) and 3 (debug)",
	               "LEVEL");

	#[cfg(unix)]
	options.optflag("f",
	                "foreground",
	                "run polaris in the foreground instead of daemonizing");

	options.optflag("h", "help", "print this help menu");

	let matches = options.parse(&args[1..])?;

	if matches.opt_present("h") {
		let program = args[0].clone();
		let brief = format!("Usage: {} [options]", program);
		print!("{}", options.usage(&brief));
		return Ok(());
	}

	let log_level = match matches.opt_str("l").as_ref().map(String::as_ref) {
		Some("0") => LogLevelFilter::Off,
		Some("1") => LogLevelFilter::Error,
		Some("2") => LogLevelFilter::Info,
		Some("3") => LogLevelFilter::Debug,
		_ => LogLevelFilter::Info,
	};

	init_log(log_level, &matches)?;

	#[cfg(unix)]
	daemonize(&matches)?;

	// Init DB
	info!("Starting up database");
	let db_path = matches.opt_str("d");
	let mut default_db_path = utils::get_data_root()?;
	default_db_path.push("db.sqlite");
	let db_path = db_path
		.map(|n| Path::new(n.as_str()).to_path_buf())
		.unwrap_or(default_db_path);
	let db = Arc::new(db::DB::new(&db_path)?);

	// Parse config
	let config_file_name = matches.opt_str("c");
	let config_file_path = config_file_name.map(|p| Path::new(p.as_str()).to_path_buf());
	if let Some(path) = config_file_path {
		let config = config::parse_toml_file(&path)?;
		config::overwrite(db.deref(), &config)?;
	}
	let config = config::read(db.deref())?;

	// Init index
	let (index_sender, index_receiver) = channel();
	let index_sender = Arc::new(Mutex::new(index_sender));
	let db_ref = db.clone();
	std::thread::spawn(move || {
		                   let db = db_ref.deref();
		                   index::update_loop(db, index_receiver);
		                  });

	// Trigger auto-indexing
	let db_ref = db.clone();
	let sender_ref = index_sender.clone();
	std::thread::spawn(move || { index::self_trigger(db_ref.deref(), sender_ref); });

	// Mount API
	let prefix_url = config.prefix_url.unwrap_or("".to_string());
	let api_url = format!("{}/api", &prefix_url);
	info!("Mounting API on {}", api_url);
	let mut mount = Mount::new();
	let handler = api::get_handler(db.clone(), index_sender)?;
	mount.mount(&api_url, handler);

	// Mount static files
	let static_url = format!("/{}", &prefix_url);

	info!("Mounting static files on {}", static_url);
	let web_dir_name = matches.opt_str("w");
	let mut default_web_dir = utils::get_data_root()?;
	default_web_dir.push("web");
	let web_dir_path = web_dir_name
		.map(|n| Path::new(n.as_str()).to_path_buf())
		.unwrap_or(default_web_dir);

	mount.mount(&static_url, Static::new(web_dir_path));

	info!("Starting up server");
	let port: u16 = matches
		.opt_str("p")
		.unwrap_or("5050".to_owned())
		.parse()
		.or(Err("invalid port number"))?;

	let mut server = match Iron::new(mount).http(("0.0.0.0", port)) {
		Ok(s) => s,
		Err(e) => bail!("Error starting up server: {}", e),
	};

	// Start DDNS updates
	let db_ref = db.clone();
	std::thread::spawn(move || { ddns::run(db_ref.deref()); });

	// Run UI
	ui::run();

	info!("Shutting down server");
	if let Err(e) = server.close() {
		bail!("Error shutting down server: {}", e);
	}

	Ok(())
}
