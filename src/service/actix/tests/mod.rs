use std::fs;
use std::path::PathBuf;

use crate::db::DB;

mod api;
mod swagger;
mod web;

fn configure_test_app(cfg: &mut actix_web::web::ServiceConfig, db_name: &str) {
	let web_url = "/";
	let web_dir_path = PathBuf::from("web");

	let swagger_url = "swagger";
	let mut swagger_dir_path = PathBuf::from("docs");
	swagger_dir_path.push("swagger");

	let mut db_path = PathBuf::new();
	db_path.push("test");
	db_path.push(format!("{}.sqlite", db_name));
	if db_path.exists() {
		fs::remove_file(&db_path).unwrap();
	}
	let db = DB::new(&db_path).unwrap();

	super::configure_app(
		cfg,
		web_url,
		web_dir_path.as_path(),
		swagger_url,
		swagger_dir_path.as_path(),
		&db,
	);
}
