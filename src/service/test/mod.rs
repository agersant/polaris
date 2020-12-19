use http::{Request, Response, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::Path;
use std::time::Duration;

pub mod constants;
pub mod protocol;

mod admin;
mod auth;
mod collection;
mod ddns;
mod lastfm;
mod media;
mod playlist;
mod settings;
mod swagger;
mod user;
mod web;

use crate::app::index;
use crate::service::dto;
use crate::service::test::constants::*;

pub use crate::service::actix::test::ServiceType;

pub trait TestService {
	fn new(test_name: &str) -> Self;
	fn fetch<T: Serialize + Clone + 'static>(&mut self, request: &Request<T>) -> Response<()>;
	fn fetch_bytes<T: Serialize + Clone + 'static>(
		&mut self,
		request: &Request<T>,
	) -> Response<Vec<u8>>;
	fn fetch_json<T: Serialize + Clone + 'static, U: DeserializeOwned>(
		&mut self,
		request: &Request<T>,
	) -> Response<U>;

	fn complete_initial_setup(&mut self) {
		let configuration = dto::Config {
			users: Some(vec![
				dto::NewUser {
					name: TEST_USERNAME_ADMIN.into(),
					password: TEST_PASSWORD_ADMIN.into(),
					admin: true,
				},
				dto::NewUser {
					name: TEST_USERNAME.into(),
					password: TEST_PASSWORD.into(),
					admin: false,
				},
			]),
			mount_dirs: Some(vec![dto::MountDir {
				name: TEST_MOUNT_NAME.into(),
				source: TEST_MOUNT_SOURCE.into(),
			}]),
			..Default::default()
		};
		let request = protocol::apply_config(configuration);
		let response = self.fetch(&request);
		assert_eq!(response.status(), StatusCode::OK);
	}

	fn login_admin(&mut self) {
		let request = protocol::login(TEST_USERNAME_ADMIN, TEST_PASSWORD_ADMIN);
		let response = self.fetch(&request);
		assert_eq!(response.status(), StatusCode::OK);
	}

	fn login(&mut self) {
		let request = protocol::login(TEST_USERNAME, TEST_PASSWORD);
		let response = self.fetch(&request);
		assert_eq!(response.status(), StatusCode::OK);
	}

	fn clear_client_cookies(&mut self);

	fn index(&mut self) {
		let request = protocol::trigger_index();
		let response = self.fetch(&request);
		assert_eq!(response.status(), StatusCode::OK);

		loop {
			let browse_request = protocol::browse(Path::new(""));
			let response = self.fetch_json::<(), Vec<index::CollectionFile>>(&browse_request);
			let entries = response.body();
			if entries.len() > 0 {
				break;
			}
			std::thread::sleep(Duration::from_secs(1));
		}

		loop {
			let flatten_request = protocol::flatten(Path::new(""));
			let response = self.fetch_json::<_, Vec<index::Song>>(&flatten_request);
			let entries = response.body();
			if entries.len() > 0 {
				break;
			}
			std::thread::sleep(Duration::from_secs(1));
		}
	}
}

fn add_trailing_slash<T>(request: &mut Request<T>) {
	*request.uri_mut() = (request.uri().to_string().trim_end_matches("/").to_string() + "/")
		.parse()
		.unwrap();
}
