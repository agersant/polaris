use actix_web;
use actix_web::client::Client;
use futures::executor::block_on;
use http::response::Response;
use http::{HeaderMap, HeaderValue};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::service::test::TestService;

pub struct ActixTestService {
	client: Client,
}

pub type ServiceType = ActixTestService;

impl TestService for ActixTestService {
	fn new(_db_name: &str) -> Self {
		ActixTestService {
			client: Client::default(),
		}
	}

	fn get(&mut self, url: &str) -> Response<()> {
		let request = self.client.get(url).send();
		let client_response = block_on(request).unwrap();
		// TODO response headers
		Response::builder()
			.status(client_response.status().as_u16())
			.body(())
			.unwrap()
	}

	fn get_bytes(&mut self, url: &str, headers: &HeaderMap<HeaderValue>) -> Response<Vec<u8>> {
		let mut request = self.client.get(url);
		for (name, value) in headers.iter() {
			request.headers_mut().insert(name.clone(), value.clone())
		}
		let request = request.send();
		let mut client_response = block_on(request).unwrap();
		let body = block_on(client_response.body()).unwrap();
		// TODO response headers
		Response::builder()
			.status(client_response.status().as_u16())
			.body(body[..].into())
			.unwrap()
	}

	fn post(&mut self, url: &str) -> Response<()> {
		let request = self.client.post(url).send();
		let client_response = block_on(request).unwrap();
		// TODO response headers
		Response::builder()
			.status(client_response.status().as_u16())
			.body(())
			.unwrap()
	}

	fn delete(&mut self, url: &str) -> Response<()> {
		let request = self.client.delete(url).send();
		let client_response = block_on(request).unwrap();
		// TODO response headers
		Response::builder()
			.status(client_response.status().as_u16())
			.body(())
			.unwrap()
	}

	fn get_json<T: DeserializeOwned>(&mut self, url: &str) -> Response<T> {
		let request = self.client.get(url).send();
		let mut client_response = block_on(request).unwrap();
		let body = block_on(client_response.json()).unwrap();
		// TODO response headers
		Response::builder()
			.status(client_response.status().as_u16())
			.body(body)
			.unwrap()
	}

	fn put_json<T: Serialize>(&mut self, url: &str, payload: &T) -> Response<()> {
		let request = self.client.put(url).send_json(payload);
		let client_response = block_on(request).unwrap();
		// TODO response headers
		Response::builder()
			.status(client_response.status().as_u16())
			.body(())
			.unwrap()
	}

	fn post_json<T: Serialize>(&mut self, url: &str, payload: &T) -> Response<()> {
		let request = self.client.post(url).send_json(payload);
		let client_response = block_on(request).unwrap();
		// TODO response headers
		Response::builder()
			.status(client_response.status().as_u16())
			.body(())
			.unwrap()
	}
}
