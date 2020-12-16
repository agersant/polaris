use actix_web::{
	client::ClientResponse,
	middleware::{Compress, Logger},
	rt::{System, SystemRunner},
	test,
	test::*,
	web::Bytes,
	App,
};
use cookie::Cookie;
use http::{header, response::Builder, Method, Request, Response};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use crate::service::actix::*;
use crate::service::test::TestService;

pub struct ActixTestService {
	system_runner: SystemRunner,
	cookies: HashMap<String, String>,
	server: TestServer,
}

pub type ServiceType = ActixTestService;

impl ActixTestService {
	fn update_cookies<T>(&mut self, actix_response: &ClientResponse<T>) {
		let cookies = actix_response.headers().get_all(header::SET_COOKIE);
		for raw_cookie in cookies {
			let cookie = Cookie::parse(raw_cookie.to_str().unwrap()).unwrap();
			self.cookies
				.insert(cookie.name().to_owned(), cookie.value().to_owned());
		}
	}

	fn process_internal<T: Serialize + Clone + 'static>(
		&mut self,
		request: &Request<T>,
	) -> (Builder, Option<Bytes>) {
		let url = request.uri().to_string();
		let body = request.body().clone();

		let mut actix_request = match *request.method() {
			Method::GET => self.server.get(url),
			Method::POST => self.server.post(url),
			Method::PUT => self.server.put(url),
			Method::DELETE => self.server.delete(url),
			_ => unimplemented!(),
		};

		for (name, value) in request.headers() {
			actix_request = actix_request.set_header(name, value.clone());
		}

		actix_request = {
			let cookies_value = self
				.cookies
				.iter()
				.map(|(name, value)| format!("{}={}", name, value))
				.collect::<Vec<_>>()
				.join("; ");
			actix_request.set_header(header::COOKIE, cookies_value)
		};

		let mut actix_response = self
			.system_runner
			.block_on(async move { actix_request.send_json(&body).await.unwrap() });

		self.update_cookies(&actix_response);

		let mut response_builder = Response::builder().status(actix_response.status());
		let headers = response_builder.headers_mut().unwrap();
		for (name, value) in actix_response.headers().iter() {
			headers.append(name, value.clone());
		}

		let is_success = actix_response.status().is_success();
		let body = if is_success {
			Some(
				self.system_runner
					.block_on(async move { actix_response.body().await.unwrap() }),
			)
		} else {
			None
		};

		(response_builder, body)
	}
}

impl TestService for ActixTestService {
	fn new(test_name: &str) -> Self {
		let mut db_path: PathBuf = ["test-output", test_name].iter().collect();
		fs::create_dir_all(&db_path).unwrap();
		db_path.push("db.sqlite");

		if db_path.exists() {
			fs::remove_file(&db_path).unwrap();
		}

		let context = service::ContextBuilder::new()
			.port(5050)
			.database_file_path(db_path)
			.web_dir_path(Path::new("test-data/web").into())
			.swagger_dir_path(["docs", "swagger"].iter().collect())
			.cache_dir_path(["test-output", test_name].iter().collect())
			.build()
			.unwrap();

		let system_runner = System::new("test");
		let server = test::start(move || {
			let config = make_config(context.clone());
			App::new()
				.wrap(Logger::default())
				.wrap(Compress::default())
				.configure(config)
		});

		ActixTestService {
			cookies: HashMap::new(),
			system_runner,
			server,
		}
	}

	fn fetch<T: Serialize + Clone + 'static>(&mut self, request: &Request<T>) -> Response<()> {
		let (response_builder, _body) = self.process_internal(request);
		response_builder.body(()).unwrap()
	}

	fn fetch_bytes<T: Serialize + Clone + 'static>(
		&mut self,
		request: &Request<T>,
	) -> Response<Vec<u8>> {
		let (response_builder, body) = self.process_internal(request);
		response_builder
			.body(body.unwrap().deref().to_owned())
			.unwrap()
	}

	fn fetch_json<T: Serialize + Clone + 'static, U: DeserializeOwned>(
		&mut self,
		request: &Request<T>,
	) -> Response<U> {
		let (response_builder, body) = self.process_internal(request);
		let body = serde_json::from_slice(&body.unwrap()).unwrap();
		response_builder.body(body).unwrap()
	}
}
