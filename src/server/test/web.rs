use http::StatusCode;

use crate::server::test::{protocol, ServiceType, TestService};
use crate::test_name;

#[tokio::test]
async fn serves_web_client() {
	let mut service = ServiceType::new(&test_name!()).await;
	let request = protocol::web_index();
	let response = service.fetch_bytes(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
}
