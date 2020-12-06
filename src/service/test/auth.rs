use dev_cookie::Cookie;
use headers::{self, HeaderMapExt};
use http::{Response, StatusCode};

use crate::service::dto;
use crate::service::test::{constants::*, protocol, ServiceType, TestService};
use crate::test_name;

fn validate_cookies<T>(response: &Response<T>) {
	let cookies: Vec<Cookie> = response
		.headers()
		.get_all(http::header::SET_COOKIE)
		.iter()
		.map(|c| Cookie::parse(c.to_str().unwrap()).unwrap())
		.collect();
	let session = cookies
		.iter()
		.find_map(|c| {
			if c.name() == dto::COOKIE_SESSION {
				Some(c.value())
			} else {
				None
			}
		})
		.unwrap();
	assert_ne!(session, TEST_USERNAME);
	assert_ne!(session, TEST_USERNAME_ADMIN);
	assert!(cookies.iter().any(|c| c.name() == dto::COOKIE_USERNAME));
	assert!(cookies.iter().any(|c| c.name() == dto::COOKIE_ADMIN));
}

fn validate_no_cookies<T>(response: &Response<T>) {
	let cookies: Vec<Cookie> = response
		.headers()
		.get_all(http::header::SET_COOKIE)
		.iter()
		.map(|c| Cookie::parse(c.to_str().unwrap()).unwrap())
		.collect();
	assert!(!cookies.iter().any(|c| c.name() == dto::COOKIE_SESSION));
	assert!(!cookies.iter().any(|c| c.name() == dto::COOKIE_USERNAME));
	assert!(!cookies.iter().any(|c| c.name() == dto::COOKIE_ADMIN));
}

#[test]
fn test_login_rejects_bad_username() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();

	let request = protocol::login("garbage", TEST_PASSWORD);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_login_rejects_bad_password() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();

	let request = protocol::login(TEST_USERNAME, "garbage");
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_login_golden_path() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();

	let request = protocol::login(TEST_USERNAME, TEST_PASSWORD);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::OK);

	validate_cookies(&response);
}

#[test]
fn test_requests_without_auth_header_do_not_set_cookies() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login();

	let request = protocol::random();
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::OK);

	validate_no_cookies(&response);
}

#[test]
fn test_authentication_via_http_header_rejects_bad_username() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();

	let mut request = protocol::random();
	let basic = headers::Authorization::basic("garbage", TEST_PASSWORD);
	request.headers_mut().typed_insert(basic);

	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_authentication_via_http_header_rejects_bad_password() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();

	let mut request = protocol::random();
	let basic = headers::Authorization::basic(TEST_PASSWORD, "garbage");
	request.headers_mut().typed_insert(basic);

	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_authentication_via_http_header_golden_path() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();

	let mut request = protocol::random();
	let basic = headers::Authorization::basic(TEST_USERNAME, TEST_PASSWORD);
	request.headers_mut().typed_insert(basic);

	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::OK);

	validate_cookies(&response);
}
