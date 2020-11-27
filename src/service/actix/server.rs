use actix_web::{
	client::Client, dev::Server, rt::System, web, web::ServiceConfig, App, HttpResponse, HttpServer,
};
use anyhow::*;
use std::path::{Path, PathBuf};

use crate::db::DB;
use crate::index::Index;
use crate::thumbnails::ThumbnailsManager;

pub fn make_config(
	port: u16,
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

	/*
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
		.mount(&api_url, api::get_routes())
		.mount(
			&swagger_url,
			StaticFiles::new(swagger_dir_path, static_file_options).rank(swagger_routes_rank),
		)
		.mount(
			&web_url,
			StaticFiles::new(web_dir_path, static_file_options).rank(web_routes_rank),
		))
		*/
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

	let auth_secret = Vec::from(auth_secret);
	let address = format!("localhost:{}", port);
	let api_url = api_url.to_owned();
	let web_url = web_url.to_owned();
	let web_dir_path = web_dir_path.to_owned();
	let swagger_url = swagger_url.to_owned();
	let swagger_dir_path = swagger_dir_path.to_owned();

	let _server = HttpServer::new(move || {
		App::new().configure(make_config(
			port,
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
