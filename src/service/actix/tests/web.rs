use actix_web::dev::Service;
use actix_web::test::TestRequest;
use actix_web::{test, App};
use function_name::named;

use super::configure_test_app;

#[named]
#[actix_rt::test]
async fn test_index() {
	let app = App::new().configure(|cfg| configure_test_app(cfg, function_name!()));
	let mut service = test::init_service(app).await;
	let req = TestRequest::get().uri("/").to_request();
	let resp = service.call(req).await.unwrap();
	assert!(resp.status().is_success());
}
