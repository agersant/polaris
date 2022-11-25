use actix_test::TestServer;
use actix_web::{
	middleware::{Compress, Logger},
	rt::{System, SystemRunner},
	web::Bytes,
	App as ActixApp,
};
use http::{response::Builder, Method, Request, Response};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::ops::Deref;

use crate::app::App;
use crate::paths::Paths;
use crate::service::actix::*;
use crate::service::dto;
use crate::service::test::TestService;
use crate::test::*;

pub struct ActixTestService {
	system_runner: SystemRunner,
	authorization: Option<dto::Authorization>,
	server: TestServer,
}

pub type ServiceType = ActixTestService;

impl ActixTestService {
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
		}
		.timeout(std::time::Duration::from_secs(30));

		for (name, value) in request.headers() {
			actix_request = actix_request.insert_header((name, value.clone()));
		}

		if let Some(ref authorization) = self.authorization {
			actix_request = actix_request.bearer_auth(&authorization.token);
		}

		let mut actix_response = self
			.system_runner
			.block_on(async move { actix_request.send_json(&body).await.unwrap() });

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
		let output_dir = prepare_test_directory(test_name);

		let paths = Paths {
			cache_dir_path: ["test-output", test_name].iter().collect(),
			config_file_path: None,
			db_file_path: output_dir.join("db.sqlite"),
			#[cfg(unix)]
			pid_file_path: output_dir.join("polaris.pid"),
			log_file_path: None,
			swagger_dir_path: ["docs", "swagger"].iter().collect(),
			web_dir_path: ["test-data", "web"].iter().collect(),
		};

		let app = App::new(5050, paths).unwrap();

		let system_runner = System::new();
		let server = actix_test::start(move || {
			let config = make_config(app.clone());
			ActixApp::new()
				.wrap(Logger::default())
				.wrap(Compress::default())
				.configure(config)
		});

		ActixTestService {
			authorization: None,
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

	fn set_authorization(&mut self, authorization: Option<dto::Authorization>) {
		self.authorization = authorization;
	}
}
