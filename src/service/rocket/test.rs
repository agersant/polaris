use rocket;
use rocket::http::Status;
use rocket::local::Client;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;

use super::server;
use crate::db::DB;
use crate::index;

pub struct TestEnvironment {
	pub client: Client,
	command_sender: Arc<index::CommandSender>,
	db: DB,
}

impl TestEnvironment {
	pub fn update_index(&self) {
		index::update(&self.db).unwrap();
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
	let db = DB::new(&db_path).unwrap();

	let web_dir_path = PathBuf::from("web");
	let mut swagger_dir_path = PathBuf::from("docs");
	swagger_dir_path.push("swagger");
	let command_sender = index::init(db.clone());

	let server = server::get_server(
		5050,
		None,
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

pub type ServiceType = TestEnvironment;

pub async fn make_service(test_name: &str) -> TestEnvironment {
	get_test_environment(&format!("{}.sqlite", test_name))
}

pub async fn get(service: &mut TestEnvironment, url: &str) {
	let client = &service.client;
	let response = client.get(url).dispatch();
	assert_eq!(response.status(), Status::Ok);
}

pub async fn get_json<T: DeserializeOwned>(service: &mut TestEnvironment, url: &str) -> T {
	let client = &service.client;
	let mut response = client.get(url).dispatch();
	assert_eq!(response.status(), Status::Ok);
	let response_body = response.body_string().unwrap();
	serde_json::from_str(&response_body).unwrap()
}

pub async fn put_json<T: Serialize>(service: &mut TestEnvironment, url: &str, payload: &T) {
	let client = &service.client;
	let body = serde_json::to_string(payload).unwrap();
	let response = client.put(url).body(&body).dispatch();
	assert_eq!(response.status(), Status::Ok);
}
