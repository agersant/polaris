use http::response::{Builder, Response};
use rocket;
use rocket::http::Status;
use rocket::local::Client;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::sync::Arc;

use super::server;
use crate::db::DB;
use crate::index;
use crate::service::test::TestService;

pub struct RocketResponse<'r, 's>(&'r mut rocket::Response<'s>);

impl<'r, 's> Into<Builder> for RocketResponse<'r, 's> {
	fn into(self) -> Builder {
		Response::builder().status(self.0.status().code)
	}
}

impl<'r, 's> Into<Response<()>> for RocketResponse<'r, 's> {
	fn into(self) -> Response<()> {
		let builder: Builder = self.into();
		builder.body(()).unwrap()
	}
}

impl<'r, 's> Into<Response<Vec<u8>>> for RocketResponse<'r, 's> {
	fn into(self) -> Response<Vec<u8>> {
		let body = self.0.body().unwrap();
		let body = body.into_bytes().unwrap();
		let builder: Builder = self.into();
		builder.body(body).unwrap()
	}
}

pub struct RocketTestService {
	client: Client,
	command_sender: Arc<index::CommandSender>,
}

pub type ServiceType = RocketTestService;

impl TestService for RocketTestService {
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

	fn get(&mut self, url: &str) -> Response<()> {
		let mut response = self.client.get(url).dispatch();
		RocketResponse(response.deref_mut()).into()
	}

	fn get_bytes(&mut self, url: &str) -> Response<Vec<u8>> {
		let mut response = self.client.get(url).dispatch();
		RocketResponse(response.deref_mut()).into()
	}

	fn post(&mut self, url: &str) -> Response<()> {
		let mut response = self.client.post(url).dispatch();
		RocketResponse(response.deref_mut()).into()
	}

	fn delete(&mut self, url: &str) -> Response<()> {
		let mut response = self.client.delete(url).dispatch();
		RocketResponse(response.deref_mut()).into()
	}

	fn get_json<T: DeserializeOwned>(&mut self, url: &str) -> T {
		let client = &self.client;
		let mut response = client.get(url).dispatch();
		assert_eq!(response.status(), Status::Ok);
		let response_body = response.body_string().unwrap();
		serde_json::from_str(&response_body).unwrap()
	}

	fn put_json<T: Serialize>(&mut self, url: &str, payload: &T) {
		let client = &self.client;
		let body = serde_json::to_string(payload).unwrap();
		let response = client.put(url).body(&body).dispatch();
		assert_eq!(response.status(), Status::Ok);
	}

	fn post_json<T: Serialize>(&mut self, url: &str, payload: &T) -> Response<()> {
		let body = serde_json::to_string(payload).unwrap();
		let mut response = self.client.post(url).body(&body).dispatch();
		RocketResponse(response.deref_mut()).into()
	}
}

impl Drop for RocketTestService {
	fn drop(&mut self) {
		self.command_sender.deref().exit().unwrap();
	}
}
