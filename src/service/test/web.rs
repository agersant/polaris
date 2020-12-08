use http::StatusCode;

use crate::service::test::{ServiceType, TestService};
use crate::test_name;

#[test]
fn test_serves_web_client() {
	let mut service = ServiceType::new(&test_name!());
	let request = service.request_builder().web_index();
	let response = service.fetch_bytes(&request);
	assert_eq!(response.status(), StatusCode::OK);
}
