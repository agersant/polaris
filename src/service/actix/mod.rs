use actix_web::{
	middleware::{normalize::TrailingSlash, Compress, Logger, NormalizePath},
	rt::System,
	web::{self, ServiceConfig},
	App, HttpServer,
};
use anyhow::*;
use log::error;

use crate::service;

mod api;

#[cfg(test)]
pub mod test;

pub fn make_config(context: service::Context) -> impl FnOnce(&mut ServiceConfig) + Clone {
	move |cfg: &mut ServiceConfig| {
		let encryption_key = cookie::Key::derive_from(&context.auth_secret[..]);
		cfg.app_data(web::Data::new(context.index))
			.app_data(web::Data::new(context.config_manager))
			.app_data(web::Data::new(context.lastfm_manager))
			.app_data(web::Data::new(context.playlist_manager))
			.app_data(web::Data::new(context.thumbnail_manager))
			.app_data(web::Data::new(context.user_manager))
			.app_data(web::Data::new(context.vfs_manager))
			.app_data(web::Data::new(encryption_key))
			.service(web::scope(&context.api_url).configure(api::make_config()))
			.service(
				actix_files::Files::new(&context.swagger_url, context.swagger_dir_path)
					.index_file("index.html"),
			)
			.service(
				actix_files::Files::new(&context.web_url, context.web_dir_path)
					.index_file("index.html"),
			);
	}
}

pub fn run(context: service::Context) -> Result<()> {
	System::run(move || {
		let address = format!("0.0.0.0:{}", context.port);
		HttpServer::new(move || {
			App::new()
				.wrap(Logger::default())
				.wrap(Compress::default())
				.wrap_fn(api::http_auth_middleware)
				.wrap(NormalizePath::new(TrailingSlash::Trim))
				.configure(make_config(context.clone()))
		})
		.disable_signals()
		.bind(address)
		.map(|server| server.run())
		.map_err(|e| error!("Error starting HTTP server: {:?}", e))
		.ok();
	})?;
	Ok(())
}
