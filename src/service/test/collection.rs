use http::StatusCode;
use std::path::{Path, PathBuf};

use crate::app::index;
use crate::service::test::{add_trailing_slash, constants::*, protocol, ServiceType, TestService};
use crate::test_name;

#[actix_web::test]
async fn browse_requires_auth() {
	let mut service = ServiceType::new(&test_name!()).await;
	let request = protocol::browse(&PathBuf::new());
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn browse_root() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let request = protocol::browse(&PathBuf::new());
	let response = service
		.fetch_json::<_, Vec<index::CollectionFile>>(&request)
		.await;
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 1);
}

#[actix_web::test]
async fn browse_directory() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let path: PathBuf = [TEST_MOUNT_NAME, "Khemmis", "Hunted"].iter().collect();
	let request = protocol::browse(&path);
	let response = service
		.fetch_json::<_, Vec<index::CollectionFile>>(&request)
		.await;
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 5);
}

#[actix_web::test]
async fn browse_bad_directory() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login().await;

	let path: PathBuf = ["not_my_collection"].iter().collect();
	let request = protocol::browse(&path);
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn flatten_requires_auth() {
	let mut service = ServiceType::new(&test_name!()).await;
	let request = protocol::flatten(&PathBuf::new());
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn flatten_root() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let request = protocol::flatten(&PathBuf::new());
	let response = service.fetch_json::<_, Vec<index::Song>>(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 13);
}

#[actix_web::test]
async fn flatten_directory() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let request = protocol::flatten(Path::new(TEST_MOUNT_NAME));
	let response = service.fetch_json::<_, Vec<index::Song>>(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 13);
}

#[actix_web::test]
async fn flatten_bad_directory() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login().await;

	let path: PathBuf = ["not_my_collection"].iter().collect();
	let request = protocol::flatten(&path);
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn random_requires_auth() {
	let mut service = ServiceType::new(&test_name!()).await;
	let request = protocol::random();
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn random_golden_path() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let request = protocol::random();
	let response = service
		.fetch_json::<_, Vec<index::Directory>>(&request)
		.await;
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 3);
}

#[actix_web::test]
async fn random_with_trailing_slash() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let mut request = protocol::random();
	add_trailing_slash(&mut request);
	let response = service
		.fetch_json::<_, Vec<index::Directory>>(&request)
		.await;
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 3);
}

#[actix_web::test]
async fn recent_requires_auth() {
	let mut service = ServiceType::new(&test_name!()).await;
	let request = protocol::recent();
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn recent_golden_path() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let request = protocol::recent();
	let response = service
		.fetch_json::<_, Vec<index::Directory>>(&request)
		.await;
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 3);
}

#[actix_web::test]
async fn recent_with_trailing_slash() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let mut request = protocol::recent();
	add_trailing_slash(&mut request);
	let response = service
		.fetch_json::<_, Vec<index::Directory>>(&request)
		.await;
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 3);
}

#[actix_web::test]
async fn search_requires_auth() {
	let mut service = ServiceType::new(&test_name!()).await;
	let request = protocol::search("");
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn search_without_query() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login().await;

	let request = protocol::search("");
	let response = service
		.fetch_json::<_, Vec<index::CollectionFile>>(&request)
		.await;
	assert_eq!(response.status(), StatusCode::OK);
}

#[actix_web::test]
async fn search_with_query() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let request = protocol::search("door");
	let response = service
		.fetch_json::<_, Vec<index::CollectionFile>>(&request)
		.await;
	let results = response.body();
	assert_eq!(results.len(), 1);
	match results[0] {
		index::CollectionFile::Song(ref s) => {
			assert_eq!(s.title, Some("Beyond The Door".into()))
		}
		_ => panic!(),
	}
}
