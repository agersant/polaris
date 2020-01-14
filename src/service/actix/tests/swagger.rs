use actix_web::dev::Service;
use actix_web::test::TestRequest;
use actix_web::{test, App};

#[actix_rt::test]
async fn test_index() {
	let app = App::new().configure(super::configure_test_app);
	let mut service = test::init_service(app).await;
	let req = TestRequest::get().uri("/swagger").to_request();
	let resp = service.call(req).await.unwrap();
	assert!(resp.status().is_success());
}

#[actix_rt::test]
async fn test_index_with_trailing_slash() {
	let app = App::new().configure(super::configure_test_app);
	let mut service = test::init_service(app).await;
	let req = TestRequest::get().uri("/swagger/").to_request();
	let resp = service.call(req).await.unwrap();
	assert!(resp.status().is_success());
}
