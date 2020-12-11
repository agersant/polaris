use http::StatusCode;

use crate::app::{config, user};
use crate::service::test::{protocol, ServiceType, TestService};
use crate::test_name;

#[test]
fn test_get_preferences_requires_auth() {
	let mut service = ServiceType::new(&test_name!());
	let request = protocol::get_preferences();
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_get_preferences_golden_path() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login();

	let request = protocol::get_preferences();
	let response = service.fetch_json::<_, user::Preferences>(&request);
	assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn test_put_preferences_requires_auth() {
	let mut service = ServiceType::new(&test_name!());
	let request = protocol::put_preferences(user::Preferences::default());
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_put_preferences_golden_path() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login();

	let request = protocol::put_preferences(user::Preferences::default());
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::OK);
}
