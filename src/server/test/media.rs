use http::{header, HeaderValue, StatusCode};
use std::path::PathBuf;

use crate::server::dto::{self, ThumbnailSize};
use crate::server::test::{constants::*, protocol, ServiceType, TestService};
use crate::test_name;

#[tokio::test]
async fn songs_requires_auth() {
	let mut service = ServiceType::new(&test_name!()).await;
	let request = protocol::songs(dto::GetSongsBulkInput::default());
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn songs_golden_path() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let valid_path =
		PathBuf::from_iter([TEST_MOUNT_NAME, "Khemmis", "Hunted", "02 - Candlelight.mp3"]);
	let invalid_path = PathBuf::from_iter(["oink.mp3"]);

	let request = protocol::songs(dto::GetSongsBulkInput {
		paths: vec![valid_path.clone(), invalid_path.clone()],
	});

	let response = service
		.fetch_json::<_, dto::GetSongsBulkOutput>(&request)
		.await;
	assert_eq!(response.status(), StatusCode::OK);

	let payload = response.body();
	assert_eq!(payload.songs[0].path, valid_path);
	assert_eq!(payload.not_found, vec![invalid_path]);
}

#[tokio::test]
async fn audio_requires_auth() {
	let mut service = ServiceType::new(&test_name!()).await;

	let path: PathBuf = [TEST_MOUNT_NAME, "Khemmis", "Hunted", "02 - Candlelight.mp3"]
		.iter()
		.collect();

	let request = protocol::audio(&path);
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn audio_golden_path() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let path: PathBuf = [TEST_MOUNT_NAME, "Khemmis", "Hunted", "02 - Candlelight.mp3"]
		.iter()
		.collect();

	let request = protocol::audio(&path);
	let response = service.fetch_bytes(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
	assert_eq!(response.body().len(), 24_142);
	assert_eq!(
		response.headers().get(header::CONTENT_LENGTH).unwrap(),
		"24142"
	);
}

#[tokio::test]
async fn audio_does_not_encode_content() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let path: PathBuf = [TEST_MOUNT_NAME, "Khemmis", "Hunted", "02 - Candlelight.mp3"]
		.iter()
		.collect();

	let mut request = protocol::audio(&path);
	let headers = request.headers_mut();
	headers.append(
		header::ACCEPT_ENCODING,
		HeaderValue::from_str("gzip, deflate, br").unwrap(),
	);

	let response = service.fetch_bytes(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
	assert_eq!(response.body().len(), 24_142);
	assert_eq!(response.headers().get(header::TRANSFER_ENCODING), None);
	assert_eq!(
		response.headers().get(header::CONTENT_LENGTH).unwrap(),
		"24142"
	);
}

#[tokio::test]
async fn audio_partial_content() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let path: PathBuf = [TEST_MOUNT_NAME, "Khemmis", "Hunted", "02 - Candlelight.mp3"]
		.iter()
		.collect();

	let mut request = protocol::audio(&path);
	let headers = request.headers_mut();
	headers.append(
		header::RANGE,
		HeaderValue::from_str("bytes=100-299").unwrap(),
	);

	let response = service.fetch_bytes(&request).await;
	assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
	assert_eq!(response.body().len(), 200);
	assert_eq!(
		response.headers().get(header::CONTENT_LENGTH).unwrap(),
		"200"
	);
}

#[tokio::test]
async fn audio_bad_path_returns_not_found() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login().await;

	let path: PathBuf = ["not_my_collection"].iter().collect();

	let request = protocol::audio(&path);
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn peaks_requires_auth() {
	let mut service = ServiceType::new(&test_name!()).await;

	let path: PathBuf = [TEST_MOUNT_NAME, "Khemmis", "Hunted", "02 - Candlelight.mp3"]
		.iter()
		.collect();

	let request = protocol::peaks(&path);
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn peaks_golden_path() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let path: PathBuf = [TEST_MOUNT_NAME, "Khemmis", "Hunted", "02 - Candlelight.mp3"]
		.iter()
		.collect();

	let request = protocol::peaks(&path);
	let response = service.fetch_bytes(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
	assert!(response.body().len() % 2 == 0);
	assert!(response.body().len() > 0);
}

#[tokio::test]
async fn peaks_bad_path_returns_not_found() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login().await;

	let path: PathBuf = ["not_my_collection"].iter().collect();

	let request = protocol::peaks(&path);
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn thumbnail_requires_auth() {
	let mut service = ServiceType::new(&test_name!()).await;

	let path: PathBuf = [TEST_MOUNT_NAME, "Khemmis", "Hunted", "Folder.jpg"]
		.iter()
		.collect();

	let size = None;
	let pad = None;
	let request = protocol::thumbnail(&path, size, pad);
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn thumbnail_golden_path() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let path: PathBuf = [TEST_MOUNT_NAME, "Khemmis", "Hunted", "Folder.jpg"]
		.iter()
		.collect();

	let size = None;
	let pad = None;
	let request = protocol::thumbnail(&path, size, pad);
	let response = service.fetch_bytes(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn thumbnail_bad_path_returns_not_found() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login().await;

	let path: PathBuf = ["not_my_collection"].iter().collect();

	let size = None;
	let pad = None;
	let request = protocol::thumbnail(&path, size, pad);
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn thumbnail_size_default() {
	thumbnail_size(&test_name!(), None, None, 400).await;
}

#[tokio::test]
async fn thumbnail_size_small() {
	thumbnail_size(&test_name!(), Some(ThumbnailSize::Small), None, 400).await;
}

#[tokio::test]
async fn thumbnail_size_large() {
	thumbnail_size(&test_name!(), Some(ThumbnailSize::Large), None, 1200).await;
}

#[tokio::test]
async fn thumbnail_size_native() {
	thumbnail_size(&test_name!(), Some(ThumbnailSize::Native), None, 1423).await;
}

async fn thumbnail_size(name: &str, size: Option<ThumbnailSize>, pad: Option<bool>, expected: u32) {
	let mut service = ServiceType::new(name).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let path: PathBuf = [TEST_MOUNT_NAME, "Tobokegao", "Picnic", "Folder.png"]
		.iter()
		.collect();

	let request = protocol::thumbnail(&path, size, pad);
	let response = service.fetch_bytes(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
	let thumbnail = image::load_from_memory(response.body()).unwrap().to_rgb8();
	assert_eq!(thumbnail.width(), expected);
	assert_eq!(thumbnail.height(), expected);
}
