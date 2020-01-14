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
	auth_secret: Option<&[u8]>,
	api_url: &str,
	web_url: &str,
	web_dir_path: &PathBuf,
	swagger_url: &str,
	swagger_dir_path: &PathBuf,
	db: Arc<DB>,
	command_sender: Arc<CommandSender>,
) -> Result<rocket::Rocket> {
	let mut config = rocket::Config::build(Environment::Production)
		.log_level(LoggingLevel::Normal)
		.port(port)
		.keep_alive(0)
		.finalize()?;

	if let Some(secret) = auth_secret {
		let encoded = base64::encode(secret);
		config.set_secret_key(encoded)?;
	}

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
