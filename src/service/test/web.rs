use crate::service::test::{constants::*, ServiceType, TestService};

#[test]
fn test_web_can_get_index() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	let request = service.request_builder().web_index();
	let _response = service.fetch_bytes(&request);
}
