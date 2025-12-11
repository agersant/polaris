use http::StatusCode;
use std::path::{Path, PathBuf};

use crate::server::dto;
use crate::server::test::protocol::V8;
use crate::server::test::{constants::*, protocol, ServiceType, TestService};
use crate::test_name;

#[tokio::test]
async fn browse_requires_auth() {
	let mut service = ServiceType::new(&test_name!()).await;
	let request = protocol::browse::<V8>(&PathBuf::new());
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn browse_root() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let request = protocol::browse::<V8>(&PathBuf::new());
	let response = service
		.fetch_json::<_, Vec<dto::BrowserEntry>>(&request)
		.await;
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert!(!entries.is_empty());
}

#[tokio::test]
async fn browse_directory() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let path: PathBuf = [TEST_MOUNT_NAME, "Khemmis", "Hunted"].iter().collect();
	let request = protocol::browse::<V8>(&path);
	let response = service
		.fetch_json::<_, Vec<dto::BrowserEntry>>(&request)
		.await;
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 5);
}

#[tokio::test]
async fn browse_missing_directory() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login().await;

	let path: PathBuf = ["not_my_collection"].iter().collect();
	let request = protocol::browse::<V8>(&path);
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn flatten_requires_auth() {
	let mut service = ServiceType::new(&test_name!()).await;
	let request = protocol::flatten::<V8>(&PathBuf::new());
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn flatten_root() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let request = protocol::flatten::<V8>(&PathBuf::new());
	let response = service.fetch_json::<_, dto::SongList>(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
	let song_list = response.body();
	assert_eq!(song_list.paths.len(), 13);
}

#[tokio::test]
async fn flatten_directory() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let request = protocol::flatten::<V8>(Path::new(TEST_MOUNT_NAME));
	let response = service.fetch_json::<_, dto::SongList>(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
	let song_list = response.body();
	assert_eq!(song_list.paths.len(), 13);
}

#[tokio::test]
async fn flatten_missing_directory() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login().await;

	let path: PathBuf = ["not_my_collection"].iter().collect();
	let request = protocol::flatten::<V8>(&path);
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
