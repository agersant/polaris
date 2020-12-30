use http::StatusCode;
use std::path::PathBuf;

use crate::service::dto;
use crate::service::test::{constants::*, protocol, ServiceType, TestService};
use crate::test_name;

#[test]
fn lastfm_scrobble_ignores_unlinked_user() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login_admin();
	service.index();
	service.login();

	let path: PathBuf = [TEST_MOUNT_NAME, "Khemmis", "Hunted", "02 - Candlelight.mp3"]
		.iter()
		.collect();

	let request = protocol::lastfm_scrobble(&path);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[test]
fn lastfm_now_playing_ignores_unlinked_user() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login_admin();
	service.index();
	service.login();

	let path: PathBuf = [TEST_MOUNT_NAME, "Khemmis", "Hunted", "02 - Candlelight.mp3"]
		.iter()
		.collect();

	let request = protocol::lastfm_now_playing(&path);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[test]
fn lastfm_link_token_requires_auth() {
	let mut service = ServiceType::new(&test_name!());
	let request = protocol::lastfm_link_token();
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn lastfm_link_token_golden_path() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login();

	let request = protocol::lastfm_link_token();
	let response = service.fetch_json::<_, dto::LastFMLinkToken>(&request);
	assert_eq!(response.status(), StatusCode::OK);
	let link_token = response.body();
	assert!(!link_token.value.is_empty());
}
