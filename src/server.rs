use rocket;
use std::path::PathBuf;
use std::sync::Arc;

use crate::db::DB;
use crate::errors;
use crate::index::CommandSender;

pub struct StaticDirs {
	pub web_dir_path: PathBuf,
	pub swagger_dir_path: PathBuf,
}

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
		.finalize()?;

	if let Some(secret) = auth_secret {
		let encoded = base64::encode(secret);
		config.set_secret_key(encoded)?;
	}

	let static_dirs = Arc::new(StaticDirs {
		web_dir_path: web_dir_path.to_path_buf(),
		swagger_dir_path: swagger_dir_path.to_path_buf(),
	});

	Ok(rocket::custom(config)
		.manage(db)
		.manage(command_sender)
		.manage(static_dirs)
		.mount(&swagger_url, crate::swagger::get_routes())
		.mount(&web_url, crate::web::get_routes())
		.mount(&api_url, crate::api::get_routes()))
}
