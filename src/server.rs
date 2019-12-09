use rocket;
use rocket_contrib::serve::StaticFiles;
use std::path::PathBuf;
use std::sync::Arc;

use crate::db::DB;
use crate::errors;
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
) -> Result<rocket::Rocket, errors::Error> {
	let mut config = rocket::Config::build(rocket::config::Environment::Production)
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
		.mount(&api_url, crate::api::get_routes())
		.mount(
			&swagger_url,
			StaticFiles::from(swagger_dir_path).rank(swagger_routes_rank),
		)
		.mount(
			&web_url,
			StaticFiles::from(web_dir_path).rank(web_routes_rank),
		))
}
