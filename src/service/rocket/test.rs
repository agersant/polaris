use http::{header::HeaderName, method::Method, response::Builder, HeaderValue, Request, Response};
use rocket;
use rocket::local::{Client, LocalResponse};
use serde::de::DeserializeOwned;
use std::fs;
use std::path::PathBuf;

use super::server;
use crate::db::DB;
use crate::index;
use crate::service::test::{protocol, Payload, TestService};
use crate::thumbnails::ThumbnailsManager;

pub struct RocketTestService {
	client: Client,
	request_builder: protocol::RequestBuilder,
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

		let mut builder = Response::builder().status(rocket_response.status().code);
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
		let request_builder = protocol::RequestBuilder::new();
		RocketTestService {
			request_builder,
			client,
		}
	}

	fn request_builder(&self) -> &protocol::RequestBuilder {
		&self.request_builder
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
}
