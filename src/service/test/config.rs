use http::StatusCode;

use crate::service::dto;
use crate::service::test::{constants::*, protocol, ServiceType, TestService};
use crate::test_name;

#[test]
fn apply_config_cannot_unadmin_self() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login_admin();

	let mut configuration = dto::Config::default();
	configuration.users = Some(vec![dto::NewUser {
		name: TEST_USERNAME_ADMIN.into(),
		password: "".into(),
		admin: false,
	}]);
	let request = protocol::apply_config(configuration);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[test]
fn apply_config_cannot_delete_self() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login_admin();

	let mut configuration = dto::Config::default();
	configuration.users = Some(vec![]);
	let request = protocol::apply_config(configuration);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::CONFLICT);
}
