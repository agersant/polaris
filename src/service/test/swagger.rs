use http::StatusCode;

use crate::service::test::{constants::*, ServiceType, TestService};

#[test]
fn test_swagger_can_get_index() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	let request = service.request_builder().swagger_index();
	let response = service.fetch_bytes(&request);
	assert_eq!(response.status(), StatusCode::OK);
}
