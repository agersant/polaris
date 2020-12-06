use actix_web::{
	middleware::{normalize::TrailingSlash, Logger, NormalizePath},
	rt::System,
	web::{self, ServiceConfig},
	App, HttpServer,
};
use anyhow::*;

use crate::service;

mod api;

#[cfg(test)]
pub mod test;

pub fn make_config(context: service::Context) -> impl FnOnce(&mut ServiceConfig) + Clone {
	move |cfg: &mut ServiceConfig| {
		let encryption_key = cookie::Key::derive_from(&context.auth_secret[..]);
		cfg.app_data(web::Data::new(context.db))
			.app_data(web::Data::new(context.index))
			.app_data(web::Data::new(context.thumbnails_manager))
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
	let system = System::new("http-server");
	let address = format!("0.0.0.0:{}", context.port);
	let _server = HttpServer::new(move || {
		App::new()
			.wrap(Logger::default())
			.wrap_fn(api::http_auth_middleware)
			.wrap(NormalizePath::new(TrailingSlash::Trim))
			.configure(make_config(context.clone()))
	})
	.bind(address)?
	.run();
	// TODO.important investigate why it takes two Ctrl+C to shutdown
	// https://github.com/actix/actix-web/issues/639 (see example at bottom)
	system.run()?;
	Ok(())
}
