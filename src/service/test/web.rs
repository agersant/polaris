use crate::service::test::{ServiceType, TestService};
use crate::unique_db_name;

#[test]
fn test_web_can_get_index() {
	let mut service = ServiceType::new(&unique_db_name!());
	let request = service.request_builder().web_index();
	let _response = service.fetch_bytes(&request);
}
