use std::path::PathBuf;

use http::StatusCode;

use crate::{
	server::{
		dto,
		test::{
			constants::*,
			protocol::{self, V8},
			ServiceType, TestService,
		},
	},
	test_name,
};

#[tokio::test]
async fn search_requires_auth() {
	let mut service = ServiceType::new(&test_name!()).await;
	let request = protocol::search::<V8>("rhapsody");
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn search_with_query() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let request = protocol::search::<V8>("door");
	let response = service.fetch_json::<_, dto::SongList>(&request).await;
	let songs = response.body();

	let path: PathBuf = [
		TEST_MOUNT_NAME,
		"Khemmis",
		"Hunted",
		"04 - Beyond The Door.mp3",
	]
	.iter()
	.collect();
	assert_eq!(songs.paths, vec![path]);
}
