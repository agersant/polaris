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
use std::fs::File;
#[cfg(unix)]
use std::io::prelude::*;
#[cfg(unix)]
use unix_daemonize::{daemonize_redirect, ChdirMode};

use anyhow::*;
use core::ops::Deref;
use getopts::Options;
use log::info;
use simplelog::{LevelFilter, SimpleLogger, TermLogger, TerminalMode};
use std::path::Path;
use std::sync::Arc;

mod config;
mod db;
mod ddns;
mod index;
mod lastfm;
mod metadata;
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
	let mut log_file = utils::get_data_root()?;
	log_file.push("polaris.log");
	let pid = match daemonize_redirect(Some(&log_file), Some(&log_file), ChdirMode::NoChdir) {
		Ok(p) => p,
		Err(e) => bail!("Daemonize error: {:#?}", e),
	};
	let mut pid_path = utils::get_data_root()?;
	pid_path.push("polaris.pid");
	let mut file = File::create(pid_path)?;
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
	info!("Starting up database");
	let db_path = matches.opt_str("d");
	let mut default_db_path = utils::get_data_root()?;
	default_db_path.push("db.sqlite");
	let db_path = db_path
		.map(|n| Path::new(n.as_str()).to_path_buf())
		.unwrap_or(default_db_path);
	let db = Arc::new(db::DB::new(&db_path)?);

	// Parse config
	info!("Parsing configuration");
	let config_file_name = matches.opt_str("c");
	let config_file_path = config_file_name.map(|p| Path::new(p.as_str()).to_path_buf());
	if let Some(path) = config_file_path {
		let config = config::parse_toml_file(&path)?;
		info!("Applying configuration");
		config::amend(db.deref(), &config)?;
	}
	let config = config::read(db.deref())?;
	let auth_secret = config::get_auth_secret(db.deref())?;

	// Init index
	info!("Initializing index");
	let command_sender = index::init(db.clone());

	// Trigger auto-indexing
	let db_auto_index = db.clone();
	let command_sender_auto_index = command_sender.clone();
	std::thread::spawn(move || {
		index::self_trigger(db_auto_index.deref(), &command_sender_auto_index);
	});

	// API mount target
	let prefix_url = config.prefix_url.unwrap_or_else(|| "".to_string());
	let api_url = format!("/{}api", &prefix_url);
	info!("Mounting API on {}", api_url);

	// Web client mount target
	let web_dir_name = matches.opt_str("w");
	let mut default_web_dir = utils::get_data_root()?;
	default_web_dir.push("web");
	let web_dir_path = web_dir_name
		.map(|n| Path::new(n.as_str()).to_path_buf())
		.unwrap_or(default_web_dir);
	info!("Static files location is {}", web_dir_path.display());
	let web_url = format!("/{}", &prefix_url);
	info!("Mounting web client files on {}", web_url);

	// Swagger files mount target
	let swagger_dir_name = matches.opt_str("s");
	let mut default_swagger_dir = utils::get_data_root()?;
	default_swagger_dir.push("swagger");
	let swagger_dir_path = swagger_dir_name
		.map(|n| Path::new(n.as_str()).to_path_buf())
		.unwrap_or(default_swagger_dir);
	info!("Swagger files location is {}", swagger_dir_path.display());
	let swagger_url = format!("/{}swagger", &prefix_url);
	info!("Mounting swagger files on {}", swagger_url);

	// Start server
	info!("Starting up server");
	let port: u16 = matches
		.opt_str("p")
		.unwrap_or_else(|| "5050".to_owned())
		.parse()
		.with_context(|| "Invalid port number")?;

	let server = service::server::get_server(
		port,
		Some(auth_secret.as_slice()),
		&api_url,
		&web_url,
		&web_dir_path,
		&swagger_url,
		&swagger_dir_path,
		db.clone(),
		command_sender,
	)?;
	std::thread::spawn(move || {
		server.launch();
	});

	// Start DDNS updates
	let db_ddns = db.clone();
	std::thread::spawn(move || {
		ddns::run(db_ddns.deref());
	});

	// Send readiness notification
	notify_ready();

	// Run UI
	ui::run();

	info!("Shutting down server");
	Ok(())
}
