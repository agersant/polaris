use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use anyhow::*;
use std::path::PathBuf;
use std::sync::Arc;

use crate::db::DB;
use crate::index::CommandSender;

async fn index() -> impl Responder {
	HttpResponse::Ok().body("hello world!")
}

#[actix_rt::main]
pub async fn run(
	port: u16,
	auth_secret: Option<&[u8]>,
	api_url: &str,
	web_url: &str,
	web_dir_path: &PathBuf,
	swagger_url: &str,
	swagger_dir_path: &PathBuf,
	db: Arc<DB>,
	command_sender: Arc<CommandSender>,
) -> Result<()> {
	let app = App::new();

	HttpServer::new(|| App::new().route("/", web::get().to(index)))
		.bind(format!("127.0.0.1:{}", port))?
		.run();

	Ok(())
}
