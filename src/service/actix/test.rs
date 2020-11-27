use actix_web::client::Client;
use actix_web::rt::System;
use http::response::Response;
use http::{HeaderMap, HeaderValue};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::service::test::TestService;

pub struct ActixTestService {}

pub type ServiceType = ActixTestService;

impl TestService for ActixTestService {
	fn new(_db_name: &str) -> Self {
		ActixTestService {}
	}

	fn get(&mut self, url: &str) -> Response<()> {
		let url = url.to_owned();
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
		let url = url.to_owned();
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
		let url = url.to_owned();
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
		let url = url.to_owned();
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
		let url = url.to_owned();
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
		let url = url.to_owned();
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
		let url = url.to_owned();
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
