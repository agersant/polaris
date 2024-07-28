use headers::{self, HeaderMapExt};
use http::StatusCode;

use crate::server::dto;
use crate::server::test::protocol::V8;
use crate::server::test::{constants::*, protocol, ServiceType, TestService};
use crate::test_name;

#[tokio::test]
async fn login_rejects_bad_username() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;

	let request = protocol::login("garbage", TEST_PASSWORD);
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_rejects_bad_password() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;

	let request = protocol::login(TEST_USERNAME, "garbage");
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_golden_path() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;

	let request = protocol::login(TEST_USERNAME, TEST_PASSWORD);
	let response = service.fetch_json::<_, dto::Authorization>(&request).await;
	assert_eq!(response.status(), StatusCode::OK);

	let authorization = response.body();
	assert_eq!(authorization.username, TEST_USERNAME);
	assert!(!authorization.is_admin);
	assert!(!authorization.token.is_empty());
}

#[tokio::test]
async fn authentication_via_bearer_http_header_rejects_bad_token() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;

	let mut request = protocol::random::<V8>();
	let bearer = headers::Authorization::bearer("garbage").unwrap();
	request.headers_mut().typed_insert(bearer);

	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn authentication_via_bearer_http_header_golden_path() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;

	let authorization = {
		let request = protocol::login(TEST_USERNAME, TEST_PASSWORD);
		let response = service.fetch_json::<_, dto::Authorization>(&request).await;
		assert_eq!(response.status(), StatusCode::OK);
		response.into_body()
	};

	service.logout().await;

	let mut request = protocol::random::<V8>();
	let bearer = headers::Authorization::bearer(&authorization.token).unwrap();
	request.headers_mut().typed_insert(bearer);
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn authentication_via_query_param_rejects_bad_token() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;

	let mut request = protocol::random::<V8>();
	*request.uri_mut() = (request.uri().to_string() + "?auth_token=garbage-token")
		.parse()
		.unwrap();

	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn authentication_via_query_param_golden_path() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;

	let authorization = {
		let request = protocol::login(TEST_USERNAME, TEST_PASSWORD);
		let response = service.fetch_json::<_, dto::Authorization>(&request).await;
		assert_eq!(response.status(), StatusCode::OK);
		response.into_body()
	};

	service.logout().await;

	let mut request = protocol::random::<V8>();
	*request.uri_mut() = format!("{}?auth_token={}", request.uri(), authorization.token)
		.parse()
		.unwrap();

	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
}
