use http::StatusCode;
use std::path::{Path, PathBuf};

use crate::index;
use crate::service::test::{add_trailing_slash, constants::*, protocol, ServiceType, TestService};
use crate::test_name;

#[test]
fn test_browse_requires_auth() {
	let mut service = ServiceType::new(&test_name!());
	let request = protocol::browse(&PathBuf::new());
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_browse_root() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login_admin();
	service.index();
	service.login();

	let request = protocol::browse(&PathBuf::new());
	let response = service.fetch_json::<_, Vec<index::CollectionFile>>(&request);
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 1);
}

#[test]
fn test_browse_directory() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login_admin();
	service.index();
	service.login();

	let path: PathBuf = [TEST_MOUNT_NAME, "Khemmis", "Hunted"].iter().collect();
	let request = protocol::browse(&path);
	let response = service.fetch_json::<_, Vec<index::CollectionFile>>(&request);
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 5);
}

#[test]
fn test_browse_bad_directory() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login();

	let path: PathBuf = ["not_my_collection"].iter().collect();
	let request = protocol::browse(&path);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn test_flatten_requires_auth() {
	let mut service = ServiceType::new(&test_name!());
	let request = protocol::flatten(&PathBuf::new());
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_flatten_root() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login_admin();
	service.index();
	service.login();

	let request = protocol::flatten(&PathBuf::new());
	let response = service.fetch_json::<_, Vec<index::Song>>(&request);
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 13);
}

#[test]
fn test_flatten_directory() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login_admin();
	service.index();
	service.login();

	let request = protocol::flatten(Path::new(TEST_MOUNT_NAME));
	let response = service.fetch_json::<_, Vec<index::Song>>(&request);
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 13);
}

#[test]
fn test_flatten_bad_directory() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login();

	let path: PathBuf = ["not_my_collection"].iter().collect();
	let request = protocol::flatten(&path);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn test_random_requires_auth() {
	let mut service = ServiceType::new(&test_name!());
	let request = protocol::random();
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_random_golden_path() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login_admin();
	service.index();
	service.login();

	let request = protocol::random();
	let response = service.fetch_json::<_, Vec<index::Directory>>(&request);
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 3);
}

#[test]
fn test_random_with_trailing_slash() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login_admin();
	service.index();
	service.login();

	let mut request = protocol::random();
	add_trailing_slash(&mut request);
	let response = service.fetch_json::<_, Vec<index::Directory>>(&request);
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 3);
}

#[test]
fn test_recent_requires_auth() {
	let mut service = ServiceType::new(&test_name!());
	let request = protocol::recent();
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_recent_golden_path() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login_admin();
	service.index();
	service.login();

	let request = protocol::recent();
	let response = service.fetch_json::<_, Vec<index::Directory>>(&request);
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 3);
}

#[test]
fn test_recent_with_trailing_slash() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login_admin();
	service.index();
	service.login();

	let mut request = protocol::recent();
	add_trailing_slash(&mut request);
	let response = service.fetch_json::<_, Vec<index::Directory>>(&request);
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 3);
}

#[test]
fn test_search_requires_auth() {
	let mut service = ServiceType::new(&test_name!());
	let request = protocol::search("");
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_search_without_query() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login();

	let request = protocol::search("");
	let response = service.fetch_json::<_, Vec<index::CollectionFile>>(&request);
	assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn test_search_with_query() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login_admin();
	service.index();
	service.login();

	let request = protocol::search("door");
	let response = service.fetch_json::<_, Vec<index::CollectionFile>>(&request);
	let results = response.body();
	assert_eq!(results.len(), 1);
	match results[0] {
		index::CollectionFile::Song(ref s) => {
			assert_eq!(s.title, Some("Beyond The Door".into()))
		}
		_ => panic!(),
	}
}
