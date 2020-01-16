use actix_web::{App, HttpServer};
use anyhow::*;
use std::path::PathBuf;
use std::sync::Arc;

use crate::db::DB;
use crate::index::CommandSender;

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
	let mut runtime = tokio::runtime::Runtime::new()?;
	let local_set = tokio::task::LocalSet::new();
	let _ = actix_rt::System::run_in_tokio("polaris_service_executor", &local_set);

	let server = HttpServer::new(move || {
		App::new().configure(|cfg| {
			super::configure_app(
				cfg,
				&web_url,
				web_dir_path.as_path(),
				&swagger_url,
				swagger_dir_path.as_path(),
				&db,
			)
		})
	})
	.bind(format!("0.0.0.0:{}", port))?
	.run();

	runtime.block_on(server).map_err(Error::new)
}
