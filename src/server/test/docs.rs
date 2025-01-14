use http::StatusCode;

use crate::server::test::{add_trailing_slash, protocol, ServiceType, TestService};
use crate::test_name;

#[tokio::test]
async fn can_get_docs_index() {
	let mut service = ServiceType::new(&test_name!()).await;
	let request = protocol::docs_index();
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn can_get_docs_index_with_trailing_slash() {
	let mut service = ServiceType::new(&test_name!()).await;
	let mut request = protocol::docs_index();
	add_trailing_slash(&mut request);
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
}
