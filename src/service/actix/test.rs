use actix_http::{Error, Request};
use actix_web::dev::{Body, ResponseBody, Service, ServiceResponse};
use actix_web::test::TestRequest;
use actix_web::{test, App};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

use crate::db::DB;

fn configure_test_app(cfg: &mut actix_web::web::ServiceConfig, db_name: &str) {
	let web_url = "/";
	let web_dir_path = PathBuf::from("web");

	let swagger_url = "swagger";
	let mut swagger_dir_path = PathBuf::from("docs");
	swagger_dir_path.push("swagger");

	let mut db_path = PathBuf::new();
	db_path.push("test");
	db_path.push(format!("{}.sqlite", db_name));
	if db_path.exists() {
		fs::remove_file(&db_path).unwrap();
	}
	let db = DB::new(&db_path).unwrap();

	super::configure_app(
		cfg,
		web_url,
		web_dir_path.as_path(),
		swagger_url,
		swagger_dir_path.as_path(),
		&db,
	);
}

pub type ServiceType = impl Service<Request = Request, Response = ServiceResponse, Error = Error>;

pub async fn make_service(test_name: &str) -> ServiceType {
	let app = App::new().configure(|cfg| configure_test_app(cfg, test_name));
	let service = test::init_service(app).await;
	service
}

pub async fn get(service: &mut ServiceType, url: &str) {
	let req = TestRequest::get().uri(url).to_request();
	let resp = service.call(req).await.unwrap();
	assert!(resp.status().is_success());
}

pub async fn get_json<T: DeserializeOwned>(service: &mut ServiceType, url: &str) -> T {
	let req = TestRequest::get().uri(url).to_request();
	let resp = service.call(req).await.unwrap();
	assert!(resp.status().is_success());
	let body = resp.response().body().as_u8();
	let response_json: T = serde_json::from_slice(body).unwrap();
	response_json
}

pub async fn put_json<T: Serialize>(service: &mut ServiceType, url: &str, payload: &T) {
	let req = TestRequest::put().uri(url).set_json(payload).to_request();
	let resp = service.call(req).await.unwrap();
	assert!(resp.status().is_success());
}

trait BodyToBytes {
	fn as_u8(&self) -> &[u8];
}

impl BodyToBytes for ResponseBody<Body> {
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
