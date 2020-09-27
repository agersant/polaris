#![recursion_limit = "256"]
#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[cfg(feature = "profile-index")]
#[macro_use]
extern crate flamer;

#[cfg(unix)]
use log::error;
#[cfg(unix)]
use sd_notify::{self, NotifyState};
#[cfg(unix)]
use std::io::prelude::*;
#[cfg(unix)]
use unix_daemonize::{daemonize_redirect, ChdirMode};

use anyhow::*;
use getopts::Options;
use log::info;
use simplelog::{LevelFilter, SimpleLogger, TermLogger, TerminalMode};
use std::fs;
use std::path::PathBuf;

mod config;
mod db;
mod ddns;
mod index;
mod lastfm;
mod playlist;
mod service;

mod thumbnails;
mod ui;
mod user;
mod utils;
mod vfs;

fn log_config() -> simplelog::Config {
	simplelog::ConfigBuilder::new()
		.set_location_level(LevelFilter::Error)
		.build()
}

#[cfg(unix)]
fn daemonize(options: &getopts::Matches) -> Result<()> {
	if options.opt_present("f") {
		return Ok(());
	}

	let log_path = matches
		.opt_str("log")
		.map(PathBuf::from)
		.unwrap_or_else(|| {
			let mut path = PathBuf::from(option_env!("POLARIS_LOG_DIR").unwrap_or("."));
			path.push("polaris.log");
			path
		});
	fs::create_dir_all(&log_path.parent().unwrap())?;

	let pid = match daemonize_redirect(Some(&log_path), Some(&log_path), ChdirMode::NoChdir) {
		Ok(p) => p,
		Err(e) => bail!("Daemonize error: {:#?}", e),
	};

	let pid_path = matches
		.opt_str("pid")
		.map(PathBuf::from)
		.unwrap_or_else(|| {
			let mut path = PathBuf::from(option_env!("POLARIS_PID_DIR").unwrap_or("."));
			path.push("polaris.pid");
			path
		});
	fs::create_dir_all(&pid_path.parent().unwrap())?;

	let mut file = fs::File::create(pid_path)?;
	file.write_all(pid.to_string().as_bytes())?;
	Ok(())
}

#[cfg(unix)]
fn init_log(log_level: LevelFilter, options: &getopts::Matches) -> Result<()> {
	if options.opt_present("f") {
		if let Err(e) = TermLogger::init(log_level, log_config(), TerminalMode::Stdout) {
			println!("Error starting terminal logger: {}", e);
		} else {
			return Ok(());
		}
	}

	if let Err(e) = SimpleLogger::init(log_level, log_config()) {
		bail!("Error starting simple logger: {}", e);
	}
	Ok(())
}

#[cfg(windows)]
fn init_log(log_level: LevelFilter, _: &getopts::Matches) -> Result<()> {
	if TermLogger::init(log_level, log_config(), TerminalMode::Stdout).is_err() {
		if let Err(e) = SimpleLogger::init(log_level, log_config()) {
			bail!("Error starting simple logger: {}", e);
		}
	};
	Ok(())
}

#[cfg(unix)]
fn notify_ready() {
	if let Ok(true) = sd_notify::booted() {
		if let Err(e) = sd_notify::notify(true, &[NotifyState::Ready]) {
			error!("Unable to send ready notification: {}", e);
		}
	}
}

#[cfg(not(unix))]
fn notify_ready() {}

fn main() -> Result<()> {
	// Parse CLI options
	let args: Vec<String> = std::env::args().collect();
	let mut options = Options::new();
	options.optopt("c", "config", "set the configuration file", "FILE");
	options.optopt("p", "port", "set polaris to run on a custom port", "PORT");
	options.optopt("d", "database", "set the path to index database", "FILE");
	options.optopt("w", "web", "set the path to web client files", "DIRECTORY");
	options.optopt("s", "swagger", "set the path to swagger files", "DIRECTORY");
	options.optopt(
		"",
		"cache",
		"set the directory to use as cache",
		"DIRECTORY",
	);
	options.optopt("", "log", "set the path to the log file", "FILE");
	options.optopt("", "pid", "set the path to the pid file", "FILE");
	options.optopt(
		"l",
		"log",
		"set the log level to a value between 0 (off) and 3 (debug)",
		"LEVEL",
	);

	#[cfg(unix)]
	options.optflag(
		"f",
		"foreground",
		"run polaris in the foreground instead of daemonizing",
	);

	options.optflag("h", "help", "print this help menu");

	let matches = options.parse(&args[1..])?;

	if matches.opt_present("h") {
		let program = args[0].clone();
		let brief = format!("Usage: {} [options]", program);
		print!("{}", options.usage(&brief));
		return Ok(());
	}

	let log_level = match matches.opt_str("l").as_ref().map(String::as_ref) {
		Some("0") => LevelFilter::Off,
		Some("1") => LevelFilter::Error,
		Some("2") => LevelFilter::Info,
		Some("3") => LevelFilter::Debug,
		_ => LevelFilter::Info,
	};

	init_log(log_level, &matches)?;

	#[cfg(unix)]
	daemonize(&matches)?;

	// Init DB
	let db_path = matches.opt_str("d").map(PathBuf::from).unwrap_or_else(|| {
		let mut path = PathBuf::from(option_env!("POLARIS_DB_DIR").unwrap_or("."));
		path.push("db.sqlite");
		path
	});
	fs::create_dir_all(&db_path.parent().unwrap())?;
	info!("Database file path is {}", db_path.display());
	let db = db::DB::new(&db_path)?;

	// Parse config
	if let Some(config_path) = matches.opt_str("c").map(PathBuf::from) {
		let config = config::parse_toml_file(&config_path)?;
		info!("Applying configuration from {}", config_path.display());
		config::amend(&db, &config)?;
	}
	let config = config::read(&db)?;
	let auth_secret = config::get_auth_secret(&db)?;

	// Locate web client files
	let web_dir_path = match matches
		.opt_str("w")
		.or(option_env!("POLARIS_WEB_DIR").map(String::from))
	{
		Some(s) => PathBuf::from(s),
		None => [".", "web"].iter().collect(),
	};
	fs::create_dir_all(&web_dir_path)?;
	info!("Static files location is {}", web_dir_path.display());

	// Locate swagger files
	let swagger_dir_path = match matches
		.opt_str("s")
		.or(option_env!("POLARIS_SWAGGER_DIR").map(String::from))
	{
		Some(s) => PathBuf::from(s),
		None => [".", "docs", "swagger"].iter().collect(),
	};
	fs::create_dir_all(&swagger_dir_path)?;
	info!("Swagger files location is {}", swagger_dir_path.display());

	// Initialize thumbnails manager
	let mut thumbnails_path = PathBuf::from(
		matches
			.opt_str("cache")
			.or(option_env!("POLARIS_CACHE_DIR").map(String::from))
			.unwrap_or(".".to_owned()),
	);
	thumbnails_path.push("thumbnails");
	fs::create_dir_all(&thumbnails_path)?;
	info!("Thumbnails location is {}", thumbnails_path.display());
	let thumbnails_manager = thumbnails::ThumbnailsManager::new(&thumbnails_path);

	// Endpoints
	let prefix_url = config.prefix_url.unwrap_or_else(|| "".to_string());
	let api_url = format!("/{}api", &prefix_url);
	let swagger_url = format!("/{}swagger", &prefix_url);
	let web_url = format!("/{}", &prefix_url);
	info!("Mounting API on {}", api_url);
	info!("Mounting web client files on {}", web_url);
	info!("Mounting swagger files on {}", swagger_url);

	// Init index
	let index = index::builder(db.clone()).periodic_updates(true).build();

	// Start DDNS updates
	let db_ddns = db.clone();
	std::thread::spawn(move || {
		ddns::run(&db_ddns);
	});

	// Start server
	info!("Starting up server");
	let port: u16 = matches
		.opt_str("p")
		.unwrap_or_else(|| "5050".to_owned())
		.parse()
		.with_context(|| "Invalid port number")?;
	let db_server = db.clone();
	std::thread::spawn(move || {
		let _ = service::server::run(
			port,
			&auth_secret,
			api_url,
			web_url,
			web_dir_path,
			swagger_url,
			swagger_dir_path,
			db_server,
			index,
			thumbnails_manager,
		);
	});

	// Send readiness notification
	notify_ready();

	// Run UI
	ui::run();

	info!("Shutting down server");
	Ok(())
}
