use actix_web::body::Body::Bytes;
use actix_web::dev::*;
use actix_web::test::TestRequest;
use actix_web::{test, App};

use crate::service::dto;

#[actix_rt::test]
async fn test_version() {
	let app = App::new().configure(super::configure_test_app);
	let mut service = test::init_service(app).await;
	let req = TestRequest::get().uri("/api/version").to_request();
	let resp = service.call(req).await.unwrap();
	assert!(resp.status().is_success());

	let body = match resp.response().body().as_ref() {
		Some(Bytes(bytes)) => bytes,
		_ => panic!("Response error"),
	};

	let response_json: dto::Version = serde_json::from_slice(body).unwrap();
	assert_eq!(response_json, dto::Version { major: 4, minor: 0 });
}
