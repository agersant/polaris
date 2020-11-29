use http::StatusCode;

use crate::config;
use crate::service::test::{constants::*, ServiceType, TestService};

#[test]
fn test_service_settings() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();

	let get_settings = service.request_builder().get_settings();

	let response = service.fetch(&get_settings);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

	service.login();

	let response = service.fetch_json::<_, config::Config>(&get_settings);
	assert_eq!(response.status(), StatusCode::OK);

	let put_settings = service
		.request_builder()
		.put_settings(config::Config::default());
	let response = service.fetch(&put_settings);
	assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn test_service_settings_cannot_unadmin_self() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();

	let mut configuration = config::Config::default();
	configuration.users = Some(vec![config::ConfigUser {
		name: TEST_USERNAME.into(),
		password: "".into(),
		admin: false,
	}]);
	let request = service.request_builder().put_settings(configuration);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::CONFLICT);
}
