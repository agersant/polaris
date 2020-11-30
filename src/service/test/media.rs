use http::{header, HeaderValue, StatusCode};
use std::path::PathBuf;

use crate::service::test::{constants::*, ServiceType, TestService};

#[test]
fn test_audio_requires_auth() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));

	let path: PathBuf = [TEST_MOUNT_NAME, "Khemmis", "Hunted", "02 - Candlelight.mp3"]
		.iter()
		.collect();

	let request = service.request_builder().audio(&path);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_audio_golden_path() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login_admin();
	service.index();
	service.login();

	let path: PathBuf = [TEST_MOUNT_NAME, "Khemmis", "Hunted", "02 - Candlelight.mp3"]
		.iter()
		.collect();

	let request = service.request_builder().audio(&path);
	let response = service.fetch_bytes(&request);
	assert_eq!(response.status(), StatusCode::OK);
	assert_eq!(response.body().len(), 24_142);
}

#[test]
fn test_audio_partial_content() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login_admin();
	service.index();
	service.login();

	let path: PathBuf = [TEST_MOUNT_NAME, "Khemmis", "Hunted", "02 - Candlelight.mp3"]
		.iter()
		.collect();

	let mut request = service.request_builder().audio(&path);
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
fn test_audio_bad_path_returns_not_found() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();

	let path: PathBuf = ["not_my_collection"].iter().collect();

	let request = service.request_builder().audio(&path);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn test_thumbnail_requires_auth() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));

	let path: PathBuf = [TEST_MOUNT_NAME, "Khemmis", "Hunted", "Folder.jpg"]
		.iter()
		.collect();

	let pad = None;
	let request = service.request_builder().thumbnail(&path, pad);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_thumbnail_golden_path() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login_admin();
	service.index();
	service.login();

	let path: PathBuf = [TEST_MOUNT_NAME, "Khemmis", "Hunted", "Folder.jpg"]
		.iter()
		.collect();

	let pad = None;
	let request = service.request_builder().thumbnail(&path, pad);
	let response = service.fetch_bytes(&request);
	assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn test_thumbnail_bad_path_returns_not_found() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();

	let path: PathBuf = ["not_my_collection"].iter().collect();

	let pad = None;
	let request = service.request_builder().thumbnail(&path, pad);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
