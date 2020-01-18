use http::response::{Builder, Response};
use http::{HeaderMap, HeaderValue};
use rocket;
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

pub struct RocketResponse<'r, 's> {
	response: &'s mut rocket::Response<'r>,
}

impl<'r, 's> RocketResponse<'r, 's> {
	fn builder(&self) -> Builder {
		let mut builder = Response::builder().status(self.response.status().code);
		for header in self.response.headers().iter() {
			builder = builder.header(header.name(), header.value());
		}
		builder
	}

	fn to_void(&self) -> Response<()> {
		let builder = self.builder();
		builder.body(()).unwrap()
	}

	fn to_bytes(&mut self) -> Response<Vec<u8>> {
		let body = self.response.body().unwrap();
		let body = body.into_bytes().unwrap();
		let builder = self.builder();
		builder.body(body).unwrap()
	}

	fn to_object<T: DeserializeOwned>(&mut self) -> Response<T> {
		let body = self.response.body_string().unwrap();
		let body = serde_json::from_str(&body).unwrap();
		let builder = self.builder();
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
		RocketResponse {
			response: response.deref_mut(),
		}
		.to_void()
	}

	fn get_bytes(&mut self, url: &str, headers: &HeaderMap<HeaderValue>) -> Response<Vec<u8>> {
		let mut request = self.client.get(url);
		for (name, value) in headers.iter() {
			request.add_header(rocket::http::Header::new(
				name.as_str().to_owned(),
				value.to_str().unwrap().to_owned(),
			))
		}
		let mut response = request.dispatch();
		RocketResponse {
			response: response.deref_mut(),
		}
		.to_bytes()
	}

	fn post(&mut self, url: &str) -> Response<()> {
		let mut response = self.client.post(url).dispatch();
		RocketResponse {
			response: response.deref_mut(),
		}
		.to_void()
	}

	fn delete(&mut self, url: &str) -> Response<()> {
		let mut response = self.client.delete(url).dispatch();
		RocketResponse {
			response: response.deref_mut(),
		}
		.to_void()
	}

	fn get_json<T: DeserializeOwned>(&mut self, url: &str) -> Response<T> {
		let mut response = self.client.get(url).dispatch();
		RocketResponse {
			response: response.deref_mut(),
		}
		.to_object()
	}

	fn put_json<T: Serialize>(&mut self, url: &str, payload: &T) -> Response<()> {
		let client = &self.client;
		let body = serde_json::to_string(payload).unwrap();
		let mut response = client.put(url).body(&body).dispatch();
		RocketResponse {
			response: response.deref_mut(),
		}
		.to_void()
	}

	fn post_json<T: Serialize>(&mut self, url: &str, payload: &T) -> Response<()> {
		let body = serde_json::to_string(payload).unwrap();
		let mut response = self.client.post(url).body(&body).dispatch();
		RocketResponse {
			response: response.deref_mut(),
		}
		.to_void()
	}
}

impl Drop for RocketTestService {
	fn drop(&mut self) {
		self.command_sender.deref().exit().unwrap();
	}
}
