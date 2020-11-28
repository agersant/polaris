use actix_web::{client::Client, rt::System, rt::SystemRunner, App, HttpServer};
use http::response::Response;
use http::{HeaderMap, HeaderValue};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread;

use super::server;
use crate::db::DB;
use crate::index;
use crate::service::test::TestService;
use crate::thumbnails::ThumbnailsManager;

lazy_static! {
	static ref NEXT_PORT_NUMBER: Mutex<u16> = Mutex::new(5000);
}

fn get_next_port_number() -> u16 {
	let mut port = NEXT_PORT_NUMBER.lock().unwrap();
	let old_port = *port;
	*port += 1;
	old_port
}

pub struct ActixTestService {
	port: u16,
	system_runner: SystemRunner,
	client: Client,
	// TODO extract cookies after each request and send them back (ugh)
}

pub type ServiceType = ActixTestService;

impl ActixTestService {
	fn build_url(&self, endpoint: &str) -> String {
		format!("http://localhost:{}{}", self.port, endpoint)
	}
}

impl TestService for ActixTestService {
	fn new(db_name: &str) -> Self {
		let port = get_next_port_number();
		let address = format!("localhost:{}", port);

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

		thread::spawn(move || {
			let system_runner = System::new("http-server");
			HttpServer::new(move || {
				let config = server::make_config(
					Vec::from(auth_secret.clone()),
					"/api".to_owned(),
					"/".to_owned(),
					web_dir_path.clone(),
					"/swagger".to_owned(),
					swagger_dir_path.clone(),
					db.clone(),
					index.clone(),
					thumbnails_manager.clone(),
				);
				App::new().configure(config)
			})
			.bind(address)
			.unwrap()
			.run();
			system_runner.run().unwrap();
		});

		let (tx, rx) = std::sync::mpsc::channel();
		let mut system_runner = System::new("main");
		system_runner.block_on(async move {
			tx.send(Client::default()).unwrap();
		});
		let client = rx.recv().unwrap();

		ActixTestService {
			system_runner,
			client,
			port,
		}
	}

	fn get(&mut self, url: &str) -> Response<()> {
		let url = self.build_url(url);
		let client = self.client.clone();
		self.system_runner.block_on(async move {
			let request = client.get(url).send();
			let client_response = request.await.unwrap();
			let mut response = Response::builder().status(client_response.status().as_u16());
			for (name, value) in client_response.headers() {
				response = response.header(name, value);
			}
			response.body(()).unwrap()
		})
	}

	fn get_bytes(&mut self, url: &str, headers: &HeaderMap<HeaderValue>) -> Response<Vec<u8>> {
		let url = self.build_url(url);
		let headers = headers.clone();
		let client = self.client.clone();
		self.system_runner.block_on(async move {
			let mut request = client.get(url);
			for (name, value) in headers.iter() {
				request.headers_mut().insert(name.clone(), value.clone())
			}
			let request = request.send();
			let mut client_response = request.await.unwrap();
			let body = client_response.body().await.unwrap();
			let mut response = Response::builder().status(client_response.status().as_u16());
			for (name, value) in client_response.headers() {
				response = response.header(name, value);
			}
			response.body(body[..].into()).unwrap()
		})
	}

	fn post(&mut self, url: &str) -> Response<()> {
		let url = self.build_url(url);
		let client = self.client.clone();
		self.system_runner.block_on(async move {
			let request = client.post(url).send();
			let client_response = request.await.unwrap();
			let mut response = Response::builder().status(client_response.status().as_u16());
			for (name, value) in client_response.headers() {
				response = response.header(name, value);
			}
			response.body(()).unwrap()
		})
	}

	fn delete(&mut self, url: &str) -> Response<()> {
		let url = self.build_url(url);
		let client = self.client.clone();
		self.system_runner.block_on(async move {
			let request = client.delete(url).send();
			let client_response = request.await.unwrap();
			let mut response = Response::builder().status(client_response.status().as_u16());
			for (name, value) in client_response.headers() {
				response = response.header(name, value);
			}
			response.body(()).unwrap()
		})
	}

	fn get_json<T: DeserializeOwned>(&mut self, url: &str) -> Response<T> {
		let url = self.build_url(url);
		let client = self.client.clone();
		self.system_runner.block_on(async move {
			let request = client.get(url).send();
			let mut client_response = request.await.unwrap();
			let body = client_response.json().await.unwrap();
			let mut response = Response::builder().status(client_response.status().as_u16());
			for (name, value) in client_response.headers() {
				response = response.header(name, value);
			}
			response.body(body).unwrap()
		})
	}

	fn put_json<T: Serialize + 'static>(&mut self, url: &str, payload: T) -> Response<()> {
		let url = self.build_url(url);
		let client = self.client.clone();
		self.system_runner.block_on(async move {
			let request = client.put(url).send_json(&payload);
			let client_response = request.await.unwrap();
			let mut response = Response::builder().status(client_response.status().as_u16());
			for (name, value) in client_response.headers() {
				response = response.header(name, value);
			}
			response.body(()).unwrap()
		})
	}

	fn post_json<T: Serialize + 'static>(&mut self, url: &str, payload: T) -> Response<()> {
		let url = self.build_url(url);
		let client = self.client.clone();
		self.system_runner.block_on(async move {
			let request = client.post(url).send_json(&payload);
			let client_response = request.await.unwrap();
			let mut response = Response::builder().status(client_response.status().as_u16());
			for (name, value) in client_response.headers() {
				response = response.header(name, value);
			}
			response.body(()).unwrap()
		})
	}
}
