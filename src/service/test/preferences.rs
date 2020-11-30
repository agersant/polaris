use http::StatusCode;

use crate::config;
use crate::service::test::{constants::*, ServiceType, TestService};

#[test]
fn test_get_preferences_requires_auth() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	let request = service.request_builder().get_preferences();
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_get_preferences_golden_path() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();

	let request = service.request_builder().get_preferences();
	let response = service.fetch_json::<_, config::Preferences>(&request);
	assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn test_put_preferences_requires_auth() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	let request = service
		.request_builder()
		.put_preferences(config::Preferences::default());
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_put_preferences_golden_path() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();

	let request = service
		.request_builder()
		.put_preferences(config::Preferences::default());
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::OK);
}
