use cookie::Cookie;
use http::StatusCode;

use crate::service::constants::*;
use crate::service::test::{constants::*, ServiceType, TestService};

#[test]
fn test_service_auth() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();

	{
		let request = service.request_builder().login("garbage", "garbage");
		let response = service.fetch(&request);
		assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
	}
	{
		let request = service.request_builder().login(TEST_USERNAME, "garbage");
		let response = service.fetch(&request);
		assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
	}
	{
		let request = service
			.request_builder()
			.login(TEST_USERNAME, TEST_PASSWORD);
		let response = service.fetch(&request);
		assert_eq!(response.status(), StatusCode::OK);

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
}
