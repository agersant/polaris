use http::StatusCode;

use crate::app::index;
use crate::service::dto;
use crate::service::test::{protocol, ServiceType, TestService};
use crate::test_name;

#[test]
fn returns_api_version() {
	let mut service = ServiceType::new(&test_name!());
	let request = protocol::version();
	let response = service.fetch_json::<_, dto::Version>(&request);
	assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn initial_setup_golden_path() {
	let mut service = ServiceType::new(&test_name!());
	let request = protocol::initial_setup();
	{
		let response = service.fetch_json::<_, dto::InitialSetup>(&request);
		assert_eq!(response.status(), StatusCode::OK);
		let initial_setup = response.body();
		assert_eq!(
			initial_setup,
			&dto::InitialSetup {
				has_any_users: false
			}
		);
	}
	service.complete_initial_setup();
	{
		let response = service.fetch_json::<_, dto::InitialSetup>(&request);
		assert_eq!(response.status(), StatusCode::OK);
		let initial_setup = response.body();
		assert_eq!(
			initial_setup,
			&dto::InitialSetup {
				has_any_users: true
			}
		);
	}
}

#[test]
fn trigger_index_golden_path() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login_admin();

	let request = protocol::random();

	let response = service.fetch_json::<_, Vec<index::Directory>>(&request);
	let entries = response.body();
	assert_eq!(entries.len(), 0);

	service.index();

	let response = service.fetch_json::<_, Vec<index::Directory>>(&request);
	let entries = response.body();
	assert_eq!(entries.len(), 3);
}

#[test]
fn trigger_index_requires_auth() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	let request = protocol::trigger_index();
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn trigger_index_requires_admin() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login();
	let request = protocol::trigger_index();
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
