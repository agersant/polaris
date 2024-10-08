use http::StatusCode;

use crate::server::dto::{self, Settings};
use crate::server::test::{protocol, ServiceType, TestService};
use crate::test_name;

#[tokio::test]
async fn get_settings_requires_auth() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;

	let request = protocol::get_settings();
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_settings_requires_admin() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login().await;

	let request = protocol::get_settings();
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn get_settings_golden_path() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;

	let request = protocol::get_settings();
	let response = service.fetch_json::<_, dto::Settings>(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn put_settings_requires_auth() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	let request = protocol::put_settings(dto::NewSettings::default());
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn put_settings_requires_admin() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login().await;
	let request = protocol::put_settings(dto::NewSettings::default());
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn put_settings_golden_path() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;

	let request = protocol::put_settings(dto::NewSettings {
		album_art_pattern: Some("test_pattern".to_owned()),
		reindex_every_n_seconds: Some(31),
		ddns_update_url: Some("ddns_url".to_owned()),
	});
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::OK);

	let request = protocol::get_settings();
	let response = service.fetch_json::<_, dto::Settings>(&request).await;
	let settings = response.body();
	assert_eq!(
		settings,
		&Settings {
			album_art_pattern: "test_pattern".to_owned(),
			reindex_every_n_seconds: 31,
			ddns_update_url: "ddns_url".to_owned(),
		},
	);
}
