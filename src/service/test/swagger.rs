use http::StatusCode;

use crate::service::test::{add_trailing_slash, ServiceType, TestService};
use crate::test_name;

#[test]
fn test_swagger_can_get_index() {
	let mut service = ServiceType::new(&test_name!());
	let request = service.request_builder().swagger_index();
	let response = service.fetch(&request);
	let status = response.status();
	assert!(status == StatusCode::OK || status == StatusCode::PERMANENT_REDIRECT);
}

#[test]
fn test_swagger_can_get_index_with_trailing_slash() {
	let mut service = ServiceType::new(&test_name!());
	let mut request = service.request_builder().swagger_index();
	add_trailing_slash(&mut request);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn test_swagger_can_get_index_with_trailing_slash() {
	let mut service = ServiceType::new(&test_name!());
	let mut request = service.request_builder().swagger_index();
	add_trailing_slash(&mut request);
	let response = service.fetch_bytes(&request);
	assert_eq!(response.status(), StatusCode::OK);
}
