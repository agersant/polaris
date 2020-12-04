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
mod media;
mod playlist;
mod preferences;
mod settings;
mod swagger;
mod web;

use crate::service::test::constants::*;
use crate::{config, index, vfs};

#[cfg(feature = "service-rocket")]
pub use crate::service::rocket::test::ServiceType;

#[cfg(feature = "service-actix")]
pub use crate::service::actix::test::ServiceType;

#[macro_export]
macro_rules! test_name {
	() => {{
		let file_name = file!();
		let file_name = file_name.replace("/", "-");
		let file_name = file_name.replace("\\", "-");
		format!("{}-line-{}", file_name, line!())
		}};
}

pub trait TestService {
	fn new(test_name: &str) -> Self;
	fn request_builder(&self) -> &protocol::RequestBuilder;
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
		let configuration = config::Config {
			album_art_pattern: None,
			reindex_every_n_seconds: None,
			ydns: None,
			users: Some(vec![
				config::ConfigUser {
					name: TEST_USERNAME_ADMIN.into(),
					password: TEST_PASSWORD_ADMIN.into(),
					admin: true,
				},
				config::ConfigUser {
					name: TEST_USERNAME.into(),
					password: TEST_PASSWORD.into(),
					admin: false,
				},
			]),
			mount_dirs: Some(vec![vfs::MountPoint {
				name: TEST_MOUNT_NAME.into(),
				source: TEST_MOUNT_SOURCE.into(),
			}]),
		};
		let request = self.request_builder().put_settings(configuration);
		let response = self.fetch(&request);
		assert_eq!(response.status(), StatusCode::OK);
	}

	fn login_admin(&mut self) {
		let request = self
			.request_builder()
			.login(TEST_USERNAME_ADMIN, TEST_PASSWORD_ADMIN);
		let response = self.fetch(&request);
		assert_eq!(response.status(), StatusCode::OK);
	}

	fn login(&mut self) {
		let request = self.request_builder().login(TEST_USERNAME, TEST_PASSWORD);
		let response = self.fetch(&request);
		assert_eq!(response.status(), StatusCode::OK);
	}

	fn index(&mut self) {
		let request = self.request_builder().trigger_index();
		let response = self.fetch(&request);
		assert_eq!(response.status(), StatusCode::OK);

		loop {
			let browse_request = self.request_builder().browse(Path::new(""));
			let response = self.fetch_json::<(), Vec<index::CollectionFile>>(&browse_request);
			let entries = response.body();
			if entries.len() > 0 {
				break;
			}
			std::thread::sleep(Duration::from_secs(1));
		}

		loop {
			let flatten_request = self.request_builder().flatten(Path::new(""));
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
