use http::{header, HeaderValue, StatusCode};
use std::path::PathBuf;

use crate::service::dto::ThumbnailSize;
use crate::service::test::{constants::*, protocol, ServiceType, TestService};
use crate::test_name;

#[test]
fn audio_requires_auth() {
	let mut service = ServiceType::new(&test_name!());

	let path: PathBuf = [TEST_MOUNT_NAME, "Khemmis", "Hunted", "02 - Candlelight.mp3"]
		.iter()
		.collect();

	let request = protocol::audio(&path);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn audio_golden_path() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login_admin();
	service.index();
	service.login();

	let path: PathBuf = [TEST_MOUNT_NAME, "Khemmis", "Hunted", "02 - Candlelight.mp3"]
		.iter()
		.collect();

	let request = protocol::audio(&path);
	let response = service.fetch_bytes(&request);
	assert_eq!(response.status(), StatusCode::OK);
	assert_eq!(response.body().len(), 24_142);
}

#[test]
fn audio_partial_content() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login_admin();
	service.index();
	service.login();

	let path: PathBuf = [TEST_MOUNT_NAME, "Khemmis", "Hunted", "02 - Candlelight.mp3"]
		.iter()
		.collect();

	let mut request = protocol::audio(&path);
	let headers = request.headers_mut();
	headers.append(
		header::RANGE,
		HeaderValue::from_str("bytes=100-299").unwrap(),
	);

	let response = service.fetch_bytes(&request);
	assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
	assert_eq!(response.body().len(), 200);
	assert_eq!(
		response.headers().get(header::CONTENT_LENGTH).unwrap(),
		"200"
	);
}

#[test]
fn audio_bad_path_returns_not_found() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login();

	let path: PathBuf = ["not_my_collection"].iter().collect();

	let request = protocol::audio(&path);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn thumbnail_requires_auth() {
	let mut service = ServiceType::new(&test_name!());

	let path: PathBuf = [TEST_MOUNT_NAME, "Khemmis", "Hunted", "Folder.jpg"]
		.iter()
		.collect();

	let size = None;
	let pad = None;
	let request = protocol::thumbnail(&path, size, pad);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn thumbnail_golden_path() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login_admin();
	service.index();
	service.login();

	let path: PathBuf = [TEST_MOUNT_NAME, "Khemmis", "Hunted", "Folder.jpg"]
		.iter()
		.collect();

	let size = None;
	let pad = None;
	let request = protocol::thumbnail(&path, size, pad);
	let response = service.fetch_bytes(&request);
	assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn thumbnail_bad_path_returns_not_found() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login();

	let path: PathBuf = ["not_my_collection"].iter().collect();

	let size = None;
	let pad = None;
	let request = protocol::thumbnail(&path, size, pad);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn thumbnail_size() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login_admin();
	service.index();
	service.login();

	let test_values = vec![
		(None, None, 400),
		(None, Some(true), 400),
		(None, Some(false), 400),
		(Some(ThumbnailSize::Small), None, 400),
		(Some(ThumbnailSize::Small), Some(true), 400),
		(Some(ThumbnailSize::Small), Some(false), 400),
		(Some(ThumbnailSize::Large), None, 1200),
		(Some(ThumbnailSize::Large), Some(true), 1200),
		(Some(ThumbnailSize::Large), Some(false), 1200),
		(Some(ThumbnailSize::Native), None, 1423),
		(Some(ThumbnailSize::Native), Some(true), 1423),
		(Some(ThumbnailSize::Native), Some(false), 1423),
	];

	let mut i = 0;
	for (size, pad, expexted) in test_values {
		let path: PathBuf = [TEST_MOUNT_NAME, "Tobokegao", "Picnic", "Folder.png"]
			.iter()
			.collect();

		let request = protocol::thumbnail(&path, size, pad);
		let response = service.fetch_bytes(&request);
		assert_eq!(response.status(), StatusCode::OK);
		let thumbnail = image::load_from_memory(response.body()).unwrap().to_rgb8();
		assert_eq!(thumbnail.width(), expexted);
		assert_eq!(thumbnail.height(), expexted);

		println!("i: {}", i);
		i += 1;
	}
}
