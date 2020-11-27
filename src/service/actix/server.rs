use actix_web::{rt::System, web, web::ServiceConfig, App, HttpServer};
use anyhow::*;
use std::path::{Path, PathBuf};

use super::api;
use crate::db::DB;
use crate::index::Index;
use crate::thumbnails::ThumbnailsManager;

pub fn make_config(
	auth_secret: Vec<u8>,
	api_url: String,
	web_url: String,
	web_dir_path: PathBuf,
	swagger_url: String,
	swagger_dir_path: PathBuf,
	db: DB,
	command_sender: Index,
	thumbnails_manager: ThumbnailsManager,
) -> impl FnOnce(&mut ServiceConfig) + Clone {
	move |cfg: &mut ServiceConfig| {
		cfg.service(web::scope(&api_url).configure(api::make_config()));
		cfg.service(
			actix_files::Files::new(&swagger_url, swagger_dir_path)
				.redirect_to_slash_directory()
				.index_file("index.html"),
		);
		cfg.service(
			actix_files::Files::new(&web_url, web_dir_path)
				.redirect_to_slash_directory()
				.index_file("index.html"),
		);
	}
}

pub fn run(
	port: u16,
	auth_secret: &[u8],
	api_url: &str,
	web_url: &str,
	web_dir_path: &Path,
	swagger_url: &str,
	swagger_dir_path: &Path,
	db: DB,
	command_sender: Index,
	thumbnails_manager: ThumbnailsManager,
) -> Result<()> {
	let system = System::new("http-server");

	// TODO group all these params into a struct
	// TODO figure out why we need so much cloning

	let auth_secret = Vec::from(auth_secret);
	let address = format!("localhost:{}", port);
	let api_url = api_url.to_owned();
	let web_url = web_url.to_owned();
	let web_dir_path = web_dir_path.to_owned();
	let swagger_url = swagger_url.to_owned();
	let swagger_dir_path = swagger_dir_path.to_owned();

	let _server = HttpServer::new(move || {
		App::new().configure(make_config(
			Vec::from(auth_secret.clone()),
			api_url.to_owned(),
			web_url.to_owned(),
			web_dir_path.clone(),
			swagger_url.to_owned(),
			swagger_dir_path.clone(),
			db.clone(),
			command_sender.clone(),
			thumbnails_manager.clone(),
		))
	})
	.bind(address)?
	.run();
	system.run()?;
	Ok(())
}
