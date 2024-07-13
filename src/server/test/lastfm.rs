use http::StatusCode;
use std::path::PathBuf;

use crate::server::dto;
use crate::server::test::{constants::*, protocol, ServiceType, TestService};
use crate::test_name;

#[tokio::test]
async fn lastfm_scrobble_ignores_unlinked_user() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let path: PathBuf = [TEST_MOUNT_NAME, "Khemmis", "Hunted", "02 - Candlelight.mp3"]
		.iter()
		.collect();

	let request = protocol::lastfm_scrobble(&path);
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn lastfm_now_playing_ignores_unlinked_user() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let path: PathBuf = [TEST_MOUNT_NAME, "Khemmis", "Hunted", "02 - Candlelight.mp3"]
		.iter()
		.collect();

	let request = protocol::lastfm_now_playing(&path);
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn lastfm_link_token_requires_auth() {
	let mut service = ServiceType::new(&test_name!()).await;
	let request = protocol::lastfm_link_token();
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn lastfm_link_token_golden_path() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login().await;

	let request = protocol::lastfm_link_token();
	let response = service
		.fetch_json::<_, dto::LastFMLinkToken>(&request)
		.await;
	assert_eq!(response.status(), StatusCode::OK);
	let link_token = response.body();
	assert!(!link_token.value.is_empty());
}
