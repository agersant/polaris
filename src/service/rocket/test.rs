use http::{header::HeaderName, method::Method, response::Builder, HeaderValue, Request, Response};
use rocket;
use rocket::local::{Client, LocalResponse};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

use crate::db::DB;
use crate::service;
use crate::service::test::{protocol, TestService};

pub struct RocketTestService {
	client: Client,
	request_builder: protocol::RequestBuilder,
}

pub type ServiceType = RocketTestService;

impl RocketTestService {
	fn process_internal<T: Serialize>(&mut self, request: &Request<T>) -> (LocalResponse, Builder) {
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

			let payload = request.body();
			let body = serde_json::to_string(payload).unwrap();
			rocket_request.set_body(body);

			let content_type = rocket::http::ContentType::JSON;
			rocket_request.add_header(content_type);

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
	fn new(test_name: &str) -> Self {
		let mut db_path: PathBuf = ["test-output", test_name].iter().collect();
		fs::create_dir_all(&db_path).unwrap();
		db_path.push("db.sqlite");

		if db_path.exists() {
			fs::remove_file(&db_path).unwrap();
		}

		let db = DB::new(&db_path).unwrap();

		let context = service::ContextBuilder::new(db)
			.web_dir_path(Path::new("web").into())
			.swagger_dir_path(["docs", "swagger"].iter().collect())
			.cache_dir_path(["test-output", test_name].iter().collect())
			.build()
			.unwrap();

		let server = service::get_server(context).unwrap();
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

	fn fetch<T: Serialize>(&mut self, request: &Request<T>) -> Response<()> {
		let (_, builder) = self.process_internal(request);
		builder.body(()).unwrap()
	}

	fn fetch_bytes<T: Serialize>(&mut self, request: &Request<T>) -> Response<Vec<u8>> {
		let (mut rocket_response, builder) = self.process_internal(request);
		let body = rocket_response.body().unwrap().into_bytes().unwrap();
		builder.body(body).unwrap()
	}

	fn fetch_json<T: Serialize, U: DeserializeOwned>(
		&mut self,
		request: &Request<T>,
	) -> Response<U> {
		let (mut rocket_response, builder) = self.process_internal(request);
		let body = rocket_response.body_string().unwrap();
		let body = serde_json::from_str(&body).unwrap();
		builder.body(body).unwrap()
	}
}
