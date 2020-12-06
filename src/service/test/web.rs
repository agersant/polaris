use crate::service::test::{protocol, ServiceType, TestService};
use crate::test_name;

#[test]
fn test_web_can_get_index() {
	let mut service = ServiceType::new(&test_name!());
	let request = protocol::web_index();
	let _response = service.fetch(&request);
}
