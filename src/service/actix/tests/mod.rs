use std::path::PathBuf;

mod api;
mod swagger;
mod web;

fn configure_test_app(cfg: &mut actix_web::web::ServiceConfig) {
	let web_url = "/";
	let web_dir_path = PathBuf::from("web");

	let swagger_url = "swagger";
	let mut swagger_dir_path = PathBuf::from("docs");
	swagger_dir_path.push("swagger");

	super::configure_app(
		cfg,
		web_url,
		web_dir_path.as_path(),
		swagger_url,
		swagger_dir_path.as_path(),
	);
}
