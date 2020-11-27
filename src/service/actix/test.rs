use actix_web::{client::Client, dev::Server, rt::System, web, App, HttpResponse, HttpServer};
use http::response::Response;
use http::{HeaderMap, HeaderValue};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::mpsc;
use std::thread;

use crate::service::test::TestService;

pub struct ActixTestService {
	port: u16,
	server: Server,
}

pub type ServiceType = ActixTestService;

impl ActixTestService {
	fn build_url(&self, endpoint: &str) -> String {
		format!("http://localhost:{}{}", self.port, endpoint)
	}
}

impl TestService for ActixTestService {
	fn new(_db_name: &str) -> Self {
		let port = 8080;
		let address = format!("localhost:{}", port);
		let (tx, rx) = mpsc::channel();
		thread::spawn(move || {
			let system = System::new("http-server");
			let server =
				HttpServer::new(|| App::new().route("/", web::get().to(|| HttpResponse::Ok())))
					.bind(address)?
					.shutdown_timeout(60) // <- Set shutdown timeout to 60 seconds
					.run();
			let _ = tx.send(server);
			system.run()
		});

		let server = rx.recv().unwrap();
		ActixTestService { server, port }
	}

	fn get(&mut self, url: &str) -> Response<()> {
		let url = self.build_url(url);
		System::new("main").block_on(async move {
			let client = Client::default();
			let request = client.get(url).send();
			let client_response = request.await.unwrap();
			// TODO response headers
			Response::builder()
				.status(client_response.status().as_u16())
				.body(())
				.unwrap()
		})
	}

	fn get_bytes(&mut self, url: &str, headers: &HeaderMap<HeaderValue>) -> Response<Vec<u8>> {
		let url = self.build_url(url);
		let headers = headers.clone();
		System::new("main").block_on(async move {
			let client = Client::default();
			let mut request = client.get(url);
			for (name, value) in headers.iter() {
				request.headers_mut().insert(name.clone(), value.clone())
			}
			let request = request.send();
			let mut client_response = request.await.unwrap();
			let body = client_response.body().await.unwrap();
			// TODO response headers
			Response::builder()
				.status(client_response.status().as_u16())
				.body(body[..].into())
				.unwrap()
		})
	}

	fn post(&mut self, url: &str) -> Response<()> {
		let url = self.build_url(url);
		System::new("main").block_on(async move {
			let client = Client::default();
			let request = client.post(url).send();
			let client_response = request.await.unwrap();
			// TODO response headers
			Response::builder()
				.status(client_response.status().as_u16())
				.body(())
				.unwrap()
		})
	}

	fn delete(&mut self, url: &str) -> Response<()> {
		let url = self.build_url(url);
		System::new("main").block_on(async move {
			let client = Client::default();
			let request = client.delete(url).send();
			let client_response = request.await.unwrap();
			// TODO response headers
			Response::builder()
				.status(client_response.status().as_u16())
				.body(())
				.unwrap()
		})
	}

	fn get_json<T: DeserializeOwned>(&mut self, url: &str) -> Response<T> {
		let url = self.build_url(url);
		System::new("main").block_on(async move {
			let client = Client::default();
			let request = client.get(url).send();
			let mut client_response = request.await.unwrap();
			let body = client_response.json().await.unwrap();
			// TODO response headers
			Response::builder()
				.status(client_response.status().as_u16())
				.body(body)
				.unwrap()
		})
	}

	fn put_json<T: Serialize>(&mut self, url: &str, payload: &T) -> Response<()> {
		let url = self.build_url(url);
		System::new("main").block_on(async move {
			let client = Client::default();
			let request = client.put(url).send(); //.send_json(payload); TODO lifetime issues
			let client_response = request.await.unwrap();
			// TODO response headers
			Response::builder()
				.status(client_response.status().as_u16())
				.body(())
				.unwrap()
		})
	}

	fn post_json<T: Serialize>(&mut self, url: &str, payload: &T) -> Response<()> {
		let url = self.build_url(url);
		System::new("main").block_on(async move {
			let client = Client::default();
			let request = client.post(url).send(); //.send_json(payload); TODO lifetime issues
			let client_response = request.await.unwrap();
			// TODO response headers
			Response::builder()
				.status(client_response.status().as_u16())
				.body(())
				.unwrap()
		})
	}
}
