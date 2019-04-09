use rocket;
use rocket::local::Client;
use std::fs;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;

use crate::db::DB;
use crate::index;
use crate::server;

pub struct TestEnvironment {
	pub client: Client,
	command_sender: Arc<index::CommandSender>,
	db: Arc<DB>,
}

impl TestEnvironment {
	pub fn update_index(&self) {
		index::update(self.db.deref()).unwrap();
	}
}

impl Drop for TestEnvironment {
	fn drop(&mut self) {
		self.command_sender.deref().exit().unwrap();
	}
}

pub fn get_test_environment(db_name: &str) -> TestEnvironment {
	let mut db_path = PathBuf::new();
	db_path.push("test");
	db_path.push(db_name);
	if db_path.exists() {
		fs::remove_file(&db_path).unwrap();
	}

	let db = Arc::new(DB::new(&db_path).unwrap());

	let web_dir_path = PathBuf::from("web");
	let mut swagger_dir_path = PathBuf::from("docs");
	swagger_dir_path.push("swagger");
	let command_sender = index::init(db.clone());

	let server = server::get_server(
		5050,
		"/api",
		"/",
		&web_dir_path,
		"/swagger",
		&swagger_dir_path,
		db.clone(),
		command_sender.clone(),
	)
	.unwrap();
	let client = Client::new(server).unwrap();
	TestEnvironment {
		client,
		command_sender,
		db,
	}
}
