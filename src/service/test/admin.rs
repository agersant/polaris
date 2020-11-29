use http::StatusCode;

use crate::index;
use crate::service::dto;
use crate::service::test::{constants::*, ServiceType, TestService};

#[test]
fn test_service_version() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	let request = service.request_builder().version();
	let response = service.fetch_json::<_, dto::Version>(&request);
	assert_eq!(response.status(), StatusCode::OK);
	let version = response.body();
	assert_eq!(version, &dto::Version { major: 5, minor: 0 });
}

#[test]
fn test_service_initial_setup() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	let request = service.request_builder().initial_setup();
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
fn test_service_trigger_index() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();

	let request = service.request_builder().random();

	let response = service.fetch_json::<_, Vec<index::Directory>>(&request);
	let entries = response.body();
	assert_eq!(entries.len(), 0);

	service.index();

	let response = service.fetch_json::<_, Vec<index::Directory>>(&request);
	let entries = response.body();
	assert_eq!(entries.len(), 3);
}
