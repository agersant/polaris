use anyhow::*;
use rocket;
use rocket::config::{Environment, LoggingLevel};
use rocket_contrib::serve::{Options, StaticFiles};
use std::path::Path;

use super::api;
use crate::db::DB;
use crate::index::Index;
use crate::thumbnails::ThumbnailsManager;

pub fn get_server(
	port: u16,
	auth_secret: &[u8],
	web_dir_path: &Path,
	swagger_dir_path: &Path,
	db: DB,
	command_sender: Index,
	thumbnails_manager: ThumbnailsManager,
) -> Result<rocket::Rocket> {
	let mut config = rocket::Config::build(Environment::Production)
		.log_level(LoggingLevel::Normal)
		.port(port)
		.keep_alive(0)
		.finalize()?;

	let encoded = base64::encode(auth_secret);
	config.set_secret_key(encoded)?;

	let swagger_routes_rank = 0;
	let web_routes_rank = swagger_routes_rank + 1;
	let static_file_options = Options::Index | Options::NormalizeDirs;

	Ok(rocket::custom(config)
		.manage(db)
		.manage(command_sender)
		.manage(thumbnails_manager)
		.mount("/api", api::get_routes())
		.mount(
			"/swagger",
			StaticFiles::new(swagger_dir_path, static_file_options).rank(swagger_routes_rank),
		)
		.mount(
			"/web",
			StaticFiles::new(web_dir_path, static_file_options).rank(web_routes_rank),
		))
}

pub fn run(
	port: u16,
	auth_secret: &[u8],
	web_dir_path: &Path,
	swagger_dir_path: &Path,
	db: DB,
	command_sender: Index,
	thumbnails_manager: ThumbnailsManager,
) -> Result<()> {
	let server = get_server(
		port,
		auth_secret,
		web_dir_path,
		swagger_dir_path,
		db,
		command_sender,
		thumbnails_manager,
	)?;
	server.launch();
	Ok(())
}
