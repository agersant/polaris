use actix_web::{rt::System, web, web::ServiceConfig, App, HttpServer};
use anyhow::*;

use crate::service;

mod api;

#[cfg(test)]
pub mod test;

pub fn make_config(context: service::Context) -> impl FnOnce(&mut ServiceConfig) + Clone {
	move |cfg: &mut ServiceConfig| {
		cfg.app_data(web::Data::new(context.db))
			.app_data(web::Data::new(context.index))
			.app_data(web::Data::new(context.thumbnails_manager))
			.service(web::scope(&context.api_url).configure(api::make_config()))
			.service(
				actix_files::Files::new(&context.swagger_url, context.swagger_dir_path)
					.redirect_to_slash_directory()
					.index_file("index.html"),
			)
			.service(
				actix_files::Files::new(&context.web_url, context.web_dir_path)
					.redirect_to_slash_directory()
					.index_file("index.html"),
			);
	}
}

pub fn run(context: service::Context) -> Result<()> {
	let system = System::new("http-server");
	let address = format!("localhost:{}", context.port);
	let _server = HttpServer::new(move || {
		App::new()
			.wrap_fn(api::http_auth_middleware)
			// TODO logger middleware
			.configure(make_config(context.clone()))
	})
	.bind(address)?
	.run();
	// TODO investigate why it takes two Ctrl+C to shutdown
	system.run()?;
	Ok(())
}
