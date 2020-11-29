use http::{
	header::HeaderName, method::Method, response::Builder, HeaderMap, HeaderValue, Request,
	Response,
};
use rocket;
use rocket::local::{Client, LocalResponse};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::ops::DerefMut;
use std::path::PathBuf;

use super::server;
use crate::db::DB;
use crate::index;
use crate::service::test::{Payload, TestService};
use crate::thumbnails::ThumbnailsManager;

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
}

pub type ServiceType = RocketTestService;

impl RocketTestService {
	fn process<T: Payload>(&mut self, request: &Request<T>) -> (LocalResponse, Builder) {
		let rocket_response = {
			let url = request.uri().to_string();
			let mut rocket_request = match *request.method() {
				Method::GET => self.client.get(url),
				Method::POST => self.client.post(url),
				Method::PUT => self.client.put(url),
				Method::DELETE => self.client.delete(url),
				_ => unimplemented!(),
			};
			for (name, value) in request.headers() {
				rocket_request.add_header(rocket::http::Header::new(
					name.as_str().to_owned(),
					value.to_str().unwrap().to_owned(),
				));
			}
			let payload = request.body().send();
			if let Some(content_type) = payload.content_type {
				if let Some(content_type) = rocket::http::ContentType::parse_flexible(content_type)
				{
					rocket_request.add_header(content_type);
				}
			}
			if let Some(content) = payload.content {
				rocket_request.set_body(content);
			}
			rocket_request.dispatch()
		};

		let mut builder = Response::builder();
		let headers = builder.headers_mut().unwrap();
		for header in rocket_response.headers().iter() {
			headers.append(
				HeaderName::from_bytes(header.name.as_str().as_bytes()).unwrap(),
				HeaderValue::from_str(header.value.as_ref()).unwrap(),
			);
		}

		(rocket_response, builder)
	}
}

impl TestService for RocketTestService {
	fn new(db_name: &str) -> Self {
		let mut db_path = PathBuf::new();
		db_path.push("test-output");
		fs::create_dir_all(&db_path).unwrap();

		db_path.push(format!("{}.sqlite", db_name));
		if db_path.exists() {
			fs::remove_file(&db_path).unwrap();
		}

		let db = DB::new(&db_path).unwrap();

		let web_dir_path = PathBuf::from("web");
		let mut swagger_dir_path = PathBuf::from("docs");
		swagger_dir_path.push("swagger");
		let index = index::builder(db.clone()).periodic_updates(false).build();

		let mut thumbnails_path = PathBuf::new();
		thumbnails_path.push("test-output");
		thumbnails_path.push("thumbnails");
		thumbnails_path.push(db_name);
		let thumbnails_manager = ThumbnailsManager::new(thumbnails_path.as_path());

		let auth_secret: [u8; 32] = [0; 32];

		let server = server::get_server(
			5050,
			&auth_secret,
			"/api",
			"/",
			&web_dir_path,
			"/swagger",
			&swagger_dir_path,
			db,
			index,
			thumbnails_manager,
		)
		.unwrap();
		let client = Client::new(server).unwrap();
		RocketTestService { client }
	}

	fn process_void<T: Payload>(&mut self, request: &Request<T>) -> Response<()> {
		let (_, builder) = self.process(request);
		builder.body(()).unwrap()
	}

	fn process_bytes<T: Payload>(&mut self, request: &Request<T>) -> Response<Vec<u8>> {
		let (mut rocket_response, builder) = self.process(request);
		let body = rocket_response.body().unwrap().into_bytes().unwrap();
		builder.body(body).unwrap()
	}

	fn process_json<T: Payload, U: DeserializeOwned>(
		&mut self,
		request: &Request<T>,
	) -> Response<U> {
		let (mut rocket_response, builder) = self.process(request);
		let body = rocket_response.body_string().unwrap();
		let body = serde_json::from_str(&body).unwrap();
		builder.body(body).unwrap()
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
