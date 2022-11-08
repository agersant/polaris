use http::StatusCode;

use crate::service::dto::{self, Settings};
use crate::service::test::{protocol, ServiceType, TestService};
use crate::test_name;

#[test]
fn get_settings_requires_auth() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();

	let request = protocol::get_settings();
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn get_settings_requires_admin() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login();

	let request = protocol::get_settings();
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[test]
fn get_settings_golden_path() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login_admin();

	let request = protocol::get_settings();
	let response = service.fetch_json::<_, dto::Settings>(&request);
	assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn put_settings_requires_auth() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	let request = protocol::put_settings(dto::NewSettings::default());
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn put_settings_requires_admin() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login();
	let request = protocol::put_settings(dto::NewSettings::default());
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[test]
fn put_settings_golden_path() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login_admin();

	let request = protocol::put_settings(dto::NewSettings {
		album_art_pattern: Some("test_pattern".to_owned()),
		reindex_every_n_seconds: Some(31),
	});
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::OK);

	let request = protocol::get_settings();
	let response = service.fetch_json::<_, dto::Settings>(&request);
	let settings = response.body();
	assert_eq!(
		settings,
		&Settings {
			album_art_pattern: "test_pattern".to_owned(),
			reindex_every_n_seconds: 31,
		},
	);
}
