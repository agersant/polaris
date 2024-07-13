use http::StatusCode;

use crate::server::test::{add_trailing_slash, protocol, ServiceType, TestService};
use crate::test_name;

#[tokio::test]
async fn can_get_swagger_index() {
	let mut service = ServiceType::new(&test_name!()).await;
	let request = protocol::swagger_index();
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn can_get_swagger_index_with_trailing_slash() {
	let mut service = ServiceType::new(&test_name!()).await;
	let mut request = protocol::swagger_index();
	add_trailing_slash(&mut request);
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
}
