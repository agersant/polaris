use cookie::Cookie;
use headers::{self, HeaderMapExt};
use http::{Response, StatusCode};

use crate::service::constants::*;
use crate::service::test::{constants::*, ServiceType, TestService};

fn validate_cookies<T>(response: &Response<T>) {
	let cookies: Vec<Cookie> = response
		.headers()
		.get_all(http::header::SET_COOKIE)
		.iter()
		.map(|c| Cookie::parse(c.to_str().unwrap()).unwrap())
		.collect();
	assert!(cookies.iter().any(|c| c.name() == COOKIE_SESSION));
	assert!(cookies.iter().any(|c| c.name() == COOKIE_USERNAME));
	assert!(cookies.iter().any(|c| c.name() == COOKIE_ADMIN));
}

#[test]
fn test_login_rejects_bad_username() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();

	let request = service.request_builder().login("garbage", TEST_PASSWORD);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_login_rejects_bad_password() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();

	let request = service.request_builder().login(TEST_USERNAME, "garbage");
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_login_golden_path() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();

	let request = service
		.request_builder()
		.login(TEST_USERNAME, TEST_PASSWORD);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::OK);

	validate_cookies(&response);
}

#[test]
fn test_authentication_via_http_header_rejects_bad_username() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();

	let mut request = service.request_builder().random();
	let basic = headers::Authorization::basic("garbage", TEST_PASSWORD);
	request.headers_mut().typed_insert(basic);

	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_authentication_via_http_header_rejects_bad_password() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();

	let mut request = service.request_builder().random();
	let basic = headers::Authorization::basic(TEST_PASSWORD, "garbage");
	request.headers_mut().typed_insert(basic);

	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_authentication_via_http_header_golden_path() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();

	let mut request = service.request_builder().random();
	let basic = headers::Authorization::basic(TEST_USERNAME, TEST_PASSWORD);
	request.headers_mut().typed_insert(basic);

	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::OK);

	validate_cookies(&response);
}
