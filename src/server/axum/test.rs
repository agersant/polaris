use axum::body::Bytes;
use axum_test::TestServer;
use http::{response::Builder, Method, Request, Response};
use serde::Serialize;

use crate::app::App;
use crate::paths::Paths;
use crate::server::axum::*;
use crate::server::dto;
use crate::server::test::TestService;
use crate::test::*;

pub struct AxumTestService {
	authorization: Option<dto::Authorization>,
	server: TestServer,
}

pub type ServiceType = AxumTestService;

impl TestService for AxumTestService {
	async fn new(test_name: &str) -> Self {
		let output_dir = prepare_test_directory(test_name);

		let paths = Paths {
			cache_dir_path: ["test-output", test_name].iter().collect(),
			config_file_path: output_dir.join("polaris.toml"),
			data_dir_path: ["test-output", test_name].iter().collect(),
			db_file_path: output_dir.join("db.sqlite"),
			#[cfg(unix)]
			pid_file_path: output_dir.join("polaris.pid"),
			log_file_path: None,
			web_dir_path: ["test-data", "web"].iter().collect(),
		};

		let app = App::new(5050, paths).await.unwrap();
		let router = make_router(app);
		let make_service = ServiceExt::<axum::extract::Request>::into_make_service(router);
		let server = TestServer::new(make_service).unwrap();

		AxumTestService {
			authorization: None,
			server,
		}
	}

	async fn execute_request<T: Serialize + Clone + 'static>(
		&mut self,
		request: &Request<T>,
	) -> (Builder, Option<Bytes>) {
		let url = request.uri().to_string();
		let body = request.body().clone();

		let mut axum_request = match *request.method() {
			Method::GET => self.server.get(&url),
			Method::POST => self.server.post(&url),
			Method::PUT => self.server.put(&url),
			Method::DELETE => self.server.delete(&url),
			_ => unimplemented!(),
		};

		for (name, value) in request.headers() {
			axum_request = axum_request.add_header(name.clone(), value.clone());
		}

		if let Some(ref authorization) = self.authorization {
			axum_request = axum_request.authorization_bearer(authorization.token.clone());
		}

		let axum_response = axum_request.json(&body).await;

		let mut response_builder = Response::builder().status(axum_response.status_code());
		let headers = response_builder.headers_mut().unwrap();
		for (name, value) in axum_response.headers().iter() {
			headers.append(name, value.clone());
		}

		let is_success = axum_response.status_code().is_success();
		let body = if is_success {
			Some(axum_response.into_bytes())
		} else {
			None
		};

		(response_builder, body)
	}

	fn set_authorization(&mut self, authorization: Option<dto::Authorization>) {
		self.authorization = authorization;
	}
}
