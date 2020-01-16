use actix_http::Request;
use actix_web::dev::*;
use actix_web::test::TestRequest;
use actix_web::{test, App};
use function_name::named;

use super::configure_test_app;
use crate::config;
use crate::service::dto;
use crate::vfs;

const TEST_USERNAME: &str = "test_user";
const TEST_PASSWORD: &str = "test_password";
const TEST_MOUNT_NAME: &str = "collection";
const TEST_MOUNT_SOURCE: &str = "test/collection";

trait BodyTest {
	fn as_u8(&self) -> &[u8];
}

impl BodyTest for ResponseBody<Body> {
	fn as_u8(&self) -> &[u8] {
		match self {
			ResponseBody::Body(ref b) => match b {
				Body::Bytes(ref by) => by.as_ref(),
				_ => panic!(),
			},
			ResponseBody::Other(ref b) => match b {
				Body::Bytes(ref by) => by.as_ref(),
				_ => panic!(),
			},
		}
	}
}

fn initial_setup() -> Request {
	let configuration = config::Config {
		album_art_pattern: None,
		prefix_url: None,
		reindex_every_n_seconds: None,
		ydns: None,
		users: Some(vec![config::ConfigUser {
			name: TEST_USERNAME.into(),
			password: TEST_PASSWORD.into(),
			admin: true,
		}]),
		mount_dirs: Some(vec![vfs::MountPoint {
			name: TEST_MOUNT_NAME.into(),
			source: TEST_MOUNT_SOURCE.into(),
		}]),
	};

	TestRequest::put()
		.uri("/api/settings")
		.set_json(&configuration)
		.to_request()
}

#[named]
#[actix_rt::test]
async fn test_version() {
	let app = App::new().configure(|cfg| configure_test_app(cfg, function_name!()));
	let mut service = test::init_service(app).await;
	let req = TestRequest::get().uri("/api/version").to_request();
	let resp = service.call(req).await.unwrap();
	assert!(resp.status().is_success());

	let body = resp.response().body().as_u8();
	let response_json: dto::Version = serde_json::from_slice(body).unwrap();
	assert_eq!(response_json, dto::Version { major: 4, minor: 0 });
}

#[named]
#[actix_rt::test]
async fn test_initial_setup() {
	let app = App::new().configure(|cfg| configure_test_app(cfg, function_name!()));
	let mut service = test::init_service(app).await;

	{
		let req = TestRequest::get().uri("/api/initial_setup").to_request();
		let resp = service.call(req).await.unwrap();
		assert!(resp.status().is_success());

		let body = resp.response().body().as_u8();
		let response_json: dto::InitialSetup = serde_json::from_slice(body).unwrap();

		assert_eq!(
			response_json,
			dto::InitialSetup {
				has_any_users: false
			}
		);
	}

	assert!(service
		.call(initial_setup())
		.await
		.unwrap()
		.status()
		.is_success());

	{
		let req = TestRequest::get().uri("/api/initial_setup").to_request();
		let resp = service.call(req).await.unwrap();
		assert!(resp.status().is_success());

		let body = resp.response().body().as_u8();
		let response_json: dto::InitialSetup = serde_json::from_slice(body).unwrap();

		assert_eq!(
			response_json,
			dto::InitialSetup {
				has_any_users: true
			}
		);
	}
}
