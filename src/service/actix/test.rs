use actix_cookie::Cookie;
use actix_web::{
	client::{Client, ClientResponse},
	middleware::{normalize::TrailingSlash, Logger, NormalizePath},
	rt::{System, SystemRunner},
	web::Bytes,
	App, HttpServer,
};
use http::{header, response::Builder, Request, Response};
use lazy_static::lazy_static;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::{fs, thread, time::Duration};

use crate::service::actix::*;
use crate::service::test::{protocol, TestService};

lazy_static! {
	 // Hundreds of consecutive unclaimned ports in the 17_000+ range
	// https://en.wikipedia.org/wiki/List_of_TCP_and_UDP_port_numbers#Well-known_ports
	static ref NEXT_PORT_NUMBER: Mutex<u16> = Mutex::new(17_000);
}

fn get_next_port_number() -> u16 {
	let mut port = NEXT_PORT_NUMBER.lock().unwrap();
	let old_port = *port;
	*port += 1;
	old_port
}

pub struct ActixTestService {
	system_runner: SystemRunner,
	cookies: HashMap<String, String>,
	request_builder: protocol::RequestBuilder,
}

pub type ServiceType = ActixTestService;

impl ActixTestService {
	fn make_client(&mut self) -> Client {
		let cookies = self.cookies.clone();
		let (tx, rx) = std::sync::mpsc::channel();
		self.system_runner.block_on(async move {
			let mut client_builder = Client::builder();
			let cookies_value = cookies
				.iter()
				.map(|(name, value)| format!("{}={}", name, value))
				.collect::<Vec<_>>()
				.join("; ");
			client_builder = client_builder
				.header(header::COOKIE, cookies_value)
				.timeout(Duration::from_secs(60));
			tx.send(client_builder.finish()).unwrap();
		});
		rx.recv().unwrap()
	}

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
		let client = self.make_client();
		let url = request.uri().to_string();
		let method = request.method().clone();
		let headers = request.headers().clone();
		let body = request.body().clone();

		let mut actix_response = self.system_runner.block_on(async move {
			let mut actix_request = client.request(method.clone(), url);
			for (name, value) in &headers {
				actix_request = actix_request.set_header(name, value.clone());
			}
			actix_request.send_json(&body).await.unwrap()
		});

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
			.port(get_next_port_number())
			.database_file_path(db_path)
			.web_dir_path(Path::new("web").into())
			.swagger_dir_path(["docs", "swagger"].iter().collect())
			.cache_dir_path(["test-output", test_name].iter().collect())
			.build()
			.unwrap();

		let address_request = format!("localhost:{}", context.port);
		let address_listen = format!("0.0.0.0:{}", context.port);

		thread::spawn(move || {
			let system_runner = System::new("http-server");
			HttpServer::new(move || {
				let config = make_config(context.clone());
				App::new()
					.wrap(Logger::default())
					.wrap_fn(api::http_auth_middleware)
					.wrap(NormalizePath::new(TrailingSlash::Trim))
					.configure(config)
			})
			.bind(address_listen)
			.unwrap()
			.run();
			system_runner.run().unwrap();
		});

		let system_runner = System::new("main");
		let request_builder = protocol::RequestBuilder::new(format!("http://{}", address_request));

		ActixTestService {
			cookies: HashMap::new(),
			system_runner,
			request_builder,
		}
	}

	fn request_builder(&self) -> &protocol::RequestBuilder {
		&self.request_builder
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
