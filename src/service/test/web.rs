use http::StatusCode;

use crate::service::test::{protocol, ServiceType, TestService};
use crate::test_name;

#[test]
fn serves_web_client() {
	let mut service = ServiceType::new(&test_name!());
	let request = protocol::web_index();
	let response = service.fetch_bytes(&request);
	assert_eq!(response.status(), StatusCode::OK);
}
