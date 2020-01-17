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
use crate::service::test::{HttpStatus, TestService};

pub struct RocketTestService {
	client: Client,
	command_sender: Arc<index::CommandSender>,
}

pub type ServiceType = RocketTestService;

impl HttpStatus for Status {
	fn is_ok(&self) -> bool {
		*self == Status::Ok
	}

	fn is_unauthorized(&self) -> bool {
		*self == Status::Unauthorized
	}
}

impl TestService for RocketTestService {
	type Status = Status;

	fn new(db_name: &str) -> Self {
		let mut db_path = PathBuf::new();
		db_path.push("test");
		db_path.push(format!("{}.sqlite", db_name));
		if db_path.exists() {
			fs::remove_file(&db_path).unwrap();
		}
		let db = DB::new(&db_path).unwrap();

		let web_dir_path = PathBuf::from("web");
		let mut swagger_dir_path = PathBuf::from("docs");
		swagger_dir_path.push("swagger");
		let command_sender = index::init(db.clone());

		let auth_secret: [u8; 32] = [0; 32];

		let server = server::get_server(
			5050,
			&auth_secret,
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
		RocketTestService {
			client,
			command_sender,
		}
	}

	fn get(&mut self, url: &str) -> Status {
		let client = &self.client;
		let response = client.get(url).dispatch();
		response.status()
	}

	fn post(&mut self, url: &str) -> Status {
		let client = &self.client;
		let response = client.post(url).dispatch();
		response.status()
	}

	fn delete(&mut self, url: &str) -> Status {
		let client = &self.client;
		let response = client.delete(url).dispatch();
		response.status()
	}

	fn get_json<T: DeserializeOwned>(&mut self, url: &str) -> T {
		let client = &self.client;
		let mut response = client.get(url).dispatch();
		assert_eq!(response.status(), Status::Ok);
		let response_body = response.body_string().unwrap();
		dbg!(&response_body);
		serde_json::from_str(&response_body).unwrap()
	}

	fn put_json<T: Serialize>(&mut self, url: &str, payload: &T) {
		let client = &self.client;
		let body = serde_json::to_string(payload).unwrap();
		let response = client.put(url).body(&body).dispatch();
		assert_eq!(response.status(), Status::Ok);
	}

	fn post_json<T: Serialize>(&mut self, url: &str, payload: &T) -> Status {
		let client = &self.client;
		let body = serde_json::to_string(payload).unwrap();
		let response = client.post(url).body(&body).dispatch();
		response.status()
	}
}

impl Drop for RocketTestService {
	fn drop(&mut self) {
		self.command_sender.deref().exit().unwrap();
	}
}
