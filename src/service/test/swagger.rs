use http::StatusCode;

use crate::service::test::{ServiceType, TestService};
use crate::test_name;

#[test]
fn test_swagger_can_get_index() {
	let mut service = ServiceType::new(&test_name!());
	let request = service.request_builder().swagger_index();
	let response = service.fetch_bytes(&request);
	assert_eq!(response.status(), StatusCode::OK);
}
