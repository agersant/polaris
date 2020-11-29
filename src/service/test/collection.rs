use http::StatusCode;
use std::path::{Path, PathBuf};

use crate::index;
use crate::service::test::{constants::*, ServiceType, TestService};

#[test]
fn test_service_browse() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();
	service.index();

	{
		let request = service.request_builder().browse(&PathBuf::new());
		let response = service.fetch_json::<_, Vec<index::CollectionFile>>(&request);
		assert_eq!(response.status(), StatusCode::OK);
		let entries = response.body();
		assert_eq!(entries.len(), 1);
	}

	{
		let path: PathBuf = ["collection", "Khemmis", "Hunted"].iter().collect();
		let request = service.request_builder().browse(&path);
		let response = service.fetch_json::<_, Vec<index::CollectionFile>>(&request);
		assert_eq!(response.status(), StatusCode::OK);
		let entries = response.body();
		assert_eq!(entries.len(), 5);
	}
}

#[test]
fn test_service_flatten() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();
	service.index();

	{
		let request = service.request_builder().flatten(&PathBuf::new());
		let response = service.fetch_json::<_, Vec<index::Song>>(&request);
		assert_eq!(response.status(), StatusCode::OK);
		let entries = response.body();
		assert_eq!(entries.len(), 13);
	}

	{
		let request = service.request_builder().flatten(Path::new("collection"));
		let response = service.fetch_json::<_, Vec<index::Song>>(&request);
		assert_eq!(response.status(), StatusCode::OK);
		let entries = response.body();
		assert_eq!(entries.len(), 13);
	}
}

#[test]
fn test_service_random() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();
	service.index();

	let request = service.request_builder().random();
	let response = service.fetch_json::<_, Vec<index::Directory>>(&request);
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 3);
}

#[test]
fn test_service_recent() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();
	service.index();

	let request = service.request_builder().recent();
	let response = service.fetch_json::<_, Vec<index::Directory>>(&request);
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 3);
}

#[test]
fn test_service_search() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();
	service.index();

	{
		let request = service.request_builder().search("");
		let response = service.fetch_json::<_, Vec<index::CollectionFile>>(&request);
		assert_eq!(response.status(), StatusCode::OK);
	}

	{
		let request = service.request_builder().search("door");
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
}
