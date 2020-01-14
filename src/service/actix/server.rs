use actix_web::{App, HttpServer};
use anyhow::*;
use std::path::PathBuf;
use std::sync::Arc;

use crate::db::DB;
use crate::index::CommandSender;

#[actix_rt::main]
pub async fn run(
	port: u16,
	auth_secret: Option<&[u8]>,
	api_url: String,
	web_url: String,
	web_dir_path: PathBuf,
	swagger_url: String,
	swagger_dir_path: PathBuf,
	db: Arc<DB>,
	command_sender: Arc<CommandSender>,
) -> Result<()> {
	HttpServer::new(move || {
		App::new().configure(|cfg| {
			super::configure_app(
				cfg,
				&web_url,
				web_dir_path.as_path(),
				&swagger_url,
				swagger_dir_path.as_path(),
			)
		})
	})
	.bind(format!("127.0.0.1:{}", port))?
	.run();

	Ok(())
}
