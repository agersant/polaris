use http::StatusCode;

use crate::service::test::{add_trailing_slash, protocol, ServiceType, TestService};
use crate::test_name;

#[test]
fn can_get_swagger_index() {
	let mut service = ServiceType::new(&test_name!());
	let request = protocol::swagger_index();
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn can_get_swagger_index_with_trailing_slash() {
	let mut service = ServiceType::new(&test_name!());
	let mut request = protocol::swagger_index();
	add_trailing_slash(&mut request);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::OK);
}
