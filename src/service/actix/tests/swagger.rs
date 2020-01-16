use actix_web::dev::Service;
use actix_web::test::TestRequest;
use actix_web::{test, App};

use super::configure_test_app;

#[actix_rt::test]
async fn test_swagger_index() {
	let app = App::new().configure(|cfg| configure_test_app(cfg, "test_swagger_index"));
	let mut service = test::init_service(app).await;
	let req = TestRequest::get().uri("/swagger").to_request();
	let resp = service.call(req).await.unwrap();
	assert!(resp.status().is_success());
}

#[actix_rt::test]
async fn test_swagger_index_with_trailing_slash() {
	let app = App::new()
		.configure(|cfg| configure_test_app(cfg, "test_swagger_index_with_trailing_slash"));
	let mut service = test::init_service(app).await;
	let req = TestRequest::get().uri("/swagger/").to_request();
	let resp = service.call(req).await.unwrap();
	assert!(resp.status().is_success());
}
