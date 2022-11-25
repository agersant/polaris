use actix_web::{
	dev::Service,
	middleware::{Compress, Logger, NormalizePath},
	rt::System,
	web::{self, ServiceConfig},
	App as ActixApp, HttpServer,
};
use log::error;

use crate::app::App;

mod api;

#[cfg(test)]
pub mod test;

pub fn make_config(app: App) -> impl FnOnce(&mut ServiceConfig) + Clone {
	move |cfg: &mut ServiceConfig| {
		cfg.app_data(web::Data::new(app.index))
			.app_data(web::Data::new(app.config_manager))
			.app_data(web::Data::new(app.ddns_manager))
			.app_data(web::Data::new(app.lastfm_manager))
			.app_data(web::Data::new(app.playlist_manager))
			.app_data(web::Data::new(app.settings_manager))
			.app_data(web::Data::new(app.thumbnail_manager))
			.app_data(web::Data::new(app.user_manager))
			.app_data(web::Data::new(app.vfs_manager))
			.service(
				web::scope("/api")
					.configure(api::make_config())
					.wrap(NormalizePath::trim()),
			)
			.service(
				actix_files::Files::new("/swagger", app.swagger_dir_path)
					.redirect_to_slash_directory()
					.index_file("index.html"),
			)
			.service(
				actix_files::Files::new("/", app.web_dir_path)
					.redirect_to_slash_directory()
					.index_file("index.html"),
			);
	}
}

pub fn run(app: App) -> Result<(), std::io::Error> {
	let address = ("0.0.0.0", app.port);
	System::new().block_on(
		HttpServer::new(move || {
			ActixApp::new()
				.wrap(Logger::default())
				.wrap_fn(|req, srv| {
					// For some reason, actix logs error as DEBUG level.
					// This logs them as ERROR level
					// See https://github.com/actix/actix-web/issues/2637
					let response_future = srv.call(req);
					async {
						let response = response_future.await?;
						if let Some(error) = response.response().error() {
							error!("{}", error);
						}
						Ok(response)
					}
				})
				.wrap(Compress::default())
				.configure(make_config(app.clone()))
		})
		.disable_signals()
		.bind(address)
		.map_err(|e| {
			error!("Error starting HTTP server: {:?}", e);
			e
		})?
		.run(),
	)
}
