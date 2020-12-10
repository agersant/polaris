use http::StatusCode;

use crate::app::config;
use crate::service::test::{constants::*, ServiceType, TestService};
use crate::test_name;

#[test]
fn test_get_settings_requires_auth() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();

	let request = service.request_builder().get_settings();
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_get_settings_requires_admin() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login();

	let request = service.request_builder().get_settings();
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[test]
fn test_get_settings_golden_path() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login_admin();

	let request = service.request_builder().get_settings();
	let response = service.fetch_json::<_, config::Config>(&request);
	assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn test_put_settings_requires_auth() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	let request = service
		.request_builder()
		.put_settings(config::Config::default());
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_put_settings_requires_admin() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login();
	let request = service
		.request_builder()
		.put_settings(config::Config::default());
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[test]
fn test_put_settings_golden_path() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login_admin();

	let request = service
		.request_builder()
		.put_settings(config::Config::default());
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn test_put_settings_cannot_unadmin_self() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login_admin();

	let mut configuration = config::Config::default();
	configuration.users = Some(vec![config::ConfigUser {
		name: TEST_USERNAME_ADMIN.into(),
		password: "".into(),
		admin: false,
	}]);
	let request = service.request_builder().put_settings(configuration);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::CONFLICT);
}
