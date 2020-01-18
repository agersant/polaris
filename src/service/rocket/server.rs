use anyhow::*;
use rocket;
use rocket::config::{Environment, LoggingLevel};
use rocket_contrib::serve::StaticFiles;
use std::path::PathBuf;
use std::sync::Arc;

use super::api;
use crate::db::DB;
use crate::index::CommandSender;

pub fn get_server(
	port: u16,
	auth_secret: &[u8],
	api_url: &str,
	web_url: &str,
	web_dir_path: &PathBuf,
	swagger_url: &str,
	swagger_dir_path: &PathBuf,
	db: DB,
	command_sender: Arc<CommandSender>,
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

	Ok(rocket::custom(config)
		.manage(db)
		.manage(command_sender)
		.mount(&api_url, api::get_routes())
		.mount(
			&swagger_url,
			StaticFiles::from(swagger_dir_path).rank(swagger_routes_rank),
		)
		.mount(
			&web_url,
			StaticFiles::from(web_dir_path).rank(web_routes_rank),
		))
}

pub fn run(
	port: u16,
	auth_secret: &[u8],
	api_url: String,
	web_url: String,
	web_dir_path: PathBuf,
	swagger_url: String,
	swagger_dir_path: PathBuf,
	db: DB,
	command_sender: Arc<CommandSender>,
) -> Result<()> {
	let server = get_server(
		port,
		auth_secret,
		&api_url,
		&web_url,
		&web_dir_path,
		&swagger_url,
		&swagger_dir_path,
		db,
		command_sender,
	)?;
	server.launch();
	Ok(())
}
