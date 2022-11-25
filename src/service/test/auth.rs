use headers::{self, HeaderMapExt};
use http::StatusCode;

use crate::service::dto;
use crate::service::test::{constants::*, protocol, ServiceType, TestService};
use crate::test_name;

#[test]
fn login_rejects_bad_username() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();

	let request = protocol::login("garbage", TEST_PASSWORD);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn login_rejects_bad_password() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();

	let request = protocol::login(TEST_USERNAME, "garbage");
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn login_golden_path() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();

	let request = protocol::login(TEST_USERNAME, TEST_PASSWORD);
	let response = service.fetch_json::<_, dto::Authorization>(&request);
	assert_eq!(response.status(), StatusCode::OK);

	let authorization = response.body();
	assert_eq!(authorization.username, TEST_USERNAME);
	assert!(!authorization.is_admin);
	assert!(!authorization.token.is_empty());
}

#[test]
fn authentication_via_bearer_http_header_rejects_bad_token() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();

	let mut request = protocol::random();
	let bearer = headers::Authorization::bearer("garbage").unwrap();
	request.headers_mut().typed_insert(bearer);

	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn authentication_via_bearer_http_header_golden_path() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();

	let authorization = {
		let request = protocol::login(TEST_USERNAME, TEST_PASSWORD);
		let response = service.fetch_json::<_, dto::Authorization>(&request);
		assert_eq!(response.status(), StatusCode::OK);
		response.into_body()
	};

	service.logout();

	let mut request = protocol::random();
	let bearer = headers::Authorization::bearer(&authorization.token).unwrap();
	request.headers_mut().typed_insert(bearer);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn authentication_via_query_param_rejects_bad_token() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();

	let mut request = protocol::random();
	*request.uri_mut() = (request.uri().to_string() + "?auth_token=garbage-token")
		.parse()
		.unwrap();

	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn authentication_via_query_param_golden_path() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();

	let authorization = {
		let request = protocol::login(TEST_USERNAME, TEST_PASSWORD);
		let response = service.fetch_json::<_, dto::Authorization>(&request);
		assert_eq!(response.status(), StatusCode::OK);
		response.into_body()
	};

	service.logout();

	let mut request = protocol::random();
	*request.uri_mut() = format!("{}?auth_token={}", request.uri(), authorization.token)
		.parse()
		.unwrap();

	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::OK);
}
