#![recursion_limit = "256"]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

use anyhow::*;
use log::{error, info};
use simplelog::{LevelFilter, SimpleLogger, TermLogger, TerminalMode};

mod app;
mod db;
mod options;
mod service;
#[cfg(test)]
mod test;
mod ui;
mod utils;

#[cfg(unix)]
fn daemonize(
	foreground: bool,
	pid_file_path: &Option<std::path::PathBuf>,
	log_file_path: &Option<std::path::PathBuf>,
) -> Result<()> {
	use std::fs;
	use std::io::Write;
	use std::path::PathBuf;
	use unix_daemonize::{daemonize_redirect, ChdirMode};

	if foreground {
		return Ok(());
	}

	let log_path = log_file_path.clone().unwrap_or_else(|| {
		let mut path = PathBuf::from(option_env!("POLARIS_LOG_DIR").unwrap_or("."));
		path.push("polaris.log");
		path
	});
	fs::create_dir_all(&log_path.parent().unwrap())?;

	let pid = match daemonize_redirect(Some(&log_path), Some(&log_path), ChdirMode::NoChdir) {
		Ok(p) => p,
		Err(e) => bail!("Daemonize error: {:#?}", e),
	};

	let pid_path = pid_file_path.clone().unwrap_or_else(|| {
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
		if let Err(e) = sd_notify::notify(true, &[sd_notify::NotifyState::Ready]) {
			error!("Unable to send ready notification: {}", e);
		}
	}
}

#[cfg(not(unix))]
fn notify_ready() {}

fn init_logging(cli_options: &options::CLIOptions) -> Result<()> {
	let log_level = cli_options.log_level.unwrap_or(LevelFilter::Info);
	let log_config = simplelog::ConfigBuilder::new()
		.set_location_level(LevelFilter::Error)
		.build();

	#[cfg(unix)]
	let prefer_term_logger = cli_options.foreground;

	#[cfg(not(unix))]
	let prefer_term_logger = true;

	if prefer_term_logger {
		match TermLogger::init(log_level, log_config.clone(), TerminalMode::Stdout) {
			Ok(_) => return Ok(()),
			Err(e) => error!("Error starting terminal logger: {}", e),
		}
	}
	SimpleLogger::init(log_level, log_config)?;
	Ok(())
}

fn main() -> Result<()> {
	// Parse CLI options
	let args: Vec<String> = std::env::args().collect();
	let options_manager = options::Manager::new();
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

	init_logging(&cli_options)?;

	// Create service context
	let mut context_builder = service::ContextBuilder::new();
	if let Some(port) = cli_options.port {
		context_builder = context_builder.port(port);
	}
	if let Some(path) = cli_options.config_file_path {
		info!("Config file location is {:#?}", path);
		context_builder = context_builder.config_file_path(path);
	}
	if let Some(path) = cli_options.database_file_path {
		context_builder = context_builder.database_file_path(path);
	}
	if let Some(path) = cli_options.web_dir_path {
		context_builder = context_builder.web_dir_path(path);
	}
	if let Some(path) = cli_options.swagger_dir_path {
		context_builder = context_builder.swagger_dir_path(path);
	}
	if let Some(path) = cli_options.cache_dir_path {
		context_builder = context_builder.cache_dir_path(path);
	}

	let context = context_builder.build()?;
	info!("Database file location is {:#?}", context.db.location());
	info!("Web client files location is {:#?}", context.web_dir_path);
	info!("Swagger files location is {:#?}", context.swagger_dir_path);
	info!(
		"Thumbnails files location is {:#?}",
		context.thumbnail_manager.get_directory()
	);

	// Begin collection scans
	context.index.begin_periodic_updates();

	// Start DDNS updates
	let ddns_manager = app::ddns::Manager::new(context.db.clone());
	std::thread::spawn(move || {
		ddns_manager.run();
	});

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
