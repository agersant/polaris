use actix_files as fs;
use actix_web::web;
use std::path::Path;

pub mod server;

#[cfg(test)]
mod tests;

fn configure_app(
	cfg: &mut web::ServiceConfig,
	web_url: &str,
	web_dir_path: &Path,
	swagger_url: &str,
	swagger_dir_path: &Path,
) {
	// TODO logging
	cfg.service(fs::Files::new(swagger_url, swagger_dir_path).index_file("index.html"))
		.service(fs::Files::new(web_url, web_dir_path).index_file("index.html"));
}
