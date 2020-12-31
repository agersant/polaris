use actix_web::{
	middleware::{normalize::TrailingSlash, Compress, Logger, NormalizePath},
	rt::System,
	web::{self, ServiceConfig},
	App, HttpServer,
};
use anyhow::*;
use log::error;

use crate::app;

mod api;

#[cfg(test)]
pub mod test;

pub fn make_config(app: app::App) -> impl FnOnce(&mut ServiceConfig) + Clone {
	move |cfg: &mut ServiceConfig| {
		let encryption_key = cookie::Key::derive_from(&app.auth_secret.key[..]);
		cfg.app_data(web::Data::new(app.index))
			.app_data(web::Data::new(app.config_manager))
			.app_data(web::Data::new(app.ddns_manager))
			.app_data(web::Data::new(app.lastfm_manager))
			.app_data(web::Data::new(app.playlist_manager))
			.app_data(web::Data::new(app.settings_manager))
			.app_data(web::Data::new(app.thumbnail_manager))
			.app_data(web::Data::new(app.user_manager))
			.app_data(web::Data::new(app.vfs_manager))
			.app_data(web::Data::new(encryption_key))
			.service(
				web::scope(&app.api_url)
					.configure(api::make_config())
					.wrap_fn(api::http_auth_middleware)
					.wrap(NormalizePath::new(TrailingSlash::Trim)),
			)
			.service(
				actix_files::Files::new(&app.swagger_url, app.swagger_dir_path)
					.redirect_to_slash_directory()
					.index_file("index.html"),
			)
			.service(
				actix_files::Files::new(&app.web_url, app.web_dir_path)
					.redirect_to_slash_directory()
					.index_file("index.html"),
			);
	}
}

pub fn run(app: app::App) -> Result<()> {
	System::run(move || {
		let address = format!("0.0.0.0:{}", app.port);
		HttpServer::new(move || {
			App::new()
				.wrap(Logger::default())
				.wrap(Compress::default())
				.configure(make_config(app.clone()))
		})
		.disable_signals()
		.bind(address)
		.map(|server| server.run())
		.map_err(|e| error!("Error starting HTTP server: {:?}", e))
		.ok();
	})?;
	Ok(())
}
