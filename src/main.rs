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
use log::info;
use simplelog::{LevelFilter, SimpleLogger, TermLogger, TerminalMode};
use std::fs;
use std::path::PathBuf;

mod artwork;
mod config;
mod db;
mod ddns;
mod index;
mod lastfm;
mod options;
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
fn daemonize(
	foreground: bool,
	pid_file_path: &Option<PathBuf>,
	log_file_path: &Option<PathBuf>,
) -> Result<()> {
	if foreground {
		return Ok(());
	}

	let log_path = log_file_path.unwrap_or_else(|| {
		let mut path = PathBuf::from(option_env!("POLARIS_LOG_DIR").unwrap_or("."));
		path.push("polaris.log");
		path
	});
	fs::create_dir_all(&log_path.parent().unwrap())?;

	let pid = match daemonize_redirect(Some(&log_path), Some(&log_path), ChdirMode::NoChdir) {
		Ok(p) => p,
		Err(e) => bail!("Daemonize error: {:#?}", e),
	};

	let pid_path = pid_file_path.unwrap_or_else(|| {
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
	let options_manager = options::OptionsManager::new();
	let cli_options = options_manager.parse(&args[1..])?;

	if cli_options.show_help {
		let program = args[0].clone();
		let brief = format!("Usage: {} [options]", program);
		print!("{}", options_manager.usage(&brief));
		return Ok(());
	}

	#[cfg(unix)]
	daemonize(
		cli_options.foreground,
		&cli_options.pid_file_path,
		&cli_options.log_file_path,
	)?;

	let log_level = cli_options.log_level.unwrap_or(LevelFilter::Info);
	// TODO validate that this works on Linux when running without -f
	if TermLogger::init(log_level, log_config(), TerminalMode::Stdout).is_err() {
		if let Err(e) = SimpleLogger::init(log_level, log_config()) {
			bail!("Error starting simple logger: {}", e);
		}
	};

	// Init DB
	let db_path = cli_options.database_file_path.clone().unwrap_or_else(|| {
		let mut path = PathBuf::from(option_env!("POLARIS_DB_DIR").unwrap_or("."));
		path.push("db.sqlite");
		path
	});
	fs::create_dir_all(&db_path.parent().unwrap())?;
	info!("Database file path is {}", db_path.display());
	let db = db::DB::new(&db_path)?;

	// Parse config
	if let Some(config_path) = cli_options.config_file_path.clone() {
		let config = config::parse_toml_file(&config_path)?;
		info!("Applying configuration from {}", config_path.display());
		config::amend(&db, &config)?;
	}
	let auth_secret = config::get_auth_secret(&db)?;

	// Start DDNS updates
	let db_ddns = db.clone();
	std::thread::spawn(move || {
		ddns::run(&db_ddns);
	});

	let mut context_builder = service::ContextBuilder::new(db).auth_secret(auth_secret);
	if let Some(port) = cli_options.port {
		context_builder = context_builder.port(port);
	}
	if let Some(web_dir_path) = cli_options.web_dir_path {
		context_builder = context_builder.web_dir_path(web_dir_path);
	}
	if let Some(swagger_dir_path) = cli_options.swagger_dir_path {
		context_builder = context_builder.swagger_dir_path(swagger_dir_path);
	}
	if let Some(cache_dir_path) = cli_options.cache_dir_path {
		context_builder = context_builder.cache_dir_path(cache_dir_path);
	}
	let context = context_builder.build()?;
	info!("Web client files location is {:#?}", context.web_dir_path);
	info!("Swagger files location is {:#?}", context.swagger_dir_path);
	info!(
		"Thumbnails files location is {:#?}",
		context.thumbnails_manager.get_directory()
	);

	context.index.begin_periodic_updates();

	// Start server
	info!("Starting up server");
	std::thread::spawn(move || {
		let _ = service::run(context);
	});

	// Send readiness notification
	notify_ready();

	// Run UI
	ui::run();

	info!("Shutting down server");
	Ok(())
}
