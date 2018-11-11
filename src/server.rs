use rocket;
use rocket_contrib::serve::StaticFiles;
use std::path::PathBuf;
use std::sync::Arc;

use crate::api;
use crate::db::DB;
use crate::errors;
use crate::index::CommandSender;

pub fn get_server(
	port: u16,
	static_url: &str,
	api_url: &str,
	web_dir_path: &PathBuf,
	db: Arc<DB>,
	command_sender: Arc<CommandSender>,
) -> Result<rocket::Rocket, errors::Error> {
	let config = rocket::Config::build(rocket::config::Environment::Production)
		.port(port)
		.finalize()?;

	Ok(rocket::custom(config)
		.manage(db)
		.manage(command_sender)
		.mount(&static_url, StaticFiles::from(web_dir_path))
		.mount(&api_url, api::get_routes()))
}
