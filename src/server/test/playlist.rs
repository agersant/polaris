use std::fs::File;
use std::io::Read;
use std::path::Path;

use http::{header, StatusCode};

use crate::server::dto::{self};
use crate::server::test::protocol::V8;
use crate::server::test::{constants::*, protocol, ServiceType, TestService};
use crate::test_name;

#[tokio::test]
async fn list_playlists_requires_auth() {
	let mut service = ServiceType::new(&test_name!()).await;
	let request = protocol::playlists();
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn list_playlists_golden_path() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login().await;
	let request = protocol::playlists();
	let response = service
		.fetch_json::<_, Vec<dto::PlaylistHeader>>(&request)
		.await;
	assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn save_playlist_requires_auth() {
	let mut service = ServiceType::new(&test_name!()).await;
	let my_playlist = dto::SavePlaylistInput { tracks: Vec::new() };
	let request = protocol::save_playlist(TEST_PLAYLIST_NAME, my_playlist);
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn save_playlist_golden_path() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login().await;

	let my_playlist = dto::SavePlaylistInput { tracks: Vec::new() };
	let request = protocol::save_playlist(TEST_PLAYLIST_NAME, my_playlist);
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn save_playlist_large() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login().await;

	let tracks = (0..100_000)
		.map(|_| Path::new("My Super Cool Song").to_owned())
		.collect();
	let my_playlist = dto::SavePlaylistInput { tracks };
	let request = protocol::save_playlist(TEST_PLAYLIST_NAME, my_playlist);
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn get_playlist_requires_auth() {
	let mut service = ServiceType::new(&test_name!()).await;
	let request = protocol::read_playlist::<V8>(TEST_PLAYLIST_NAME);
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_playlist_golden_path() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	{
		let my_playlist = dto::SavePlaylistInput {
			#[rustfmt::skip]
			tracks: vec![
				[TEST_MOUNT_NAME, "Khemmis", "Hunted", "02 - Candlelight.mp3"].iter().collect(),
				[TEST_MOUNT_NAME, "Khemmis", "Hunted", "05 - Hunted.mp3"].iter().collect(),
			],
		};
		let request = protocol::save_playlist(TEST_PLAYLIST_NAME, my_playlist);
		let response = service.fetch(&request).await;
		assert_eq!(response.status(), StatusCode::OK);
	}

	let request = protocol::read_playlist::<V8>(TEST_PLAYLIST_NAME);
	let response = service.fetch_json::<_, dto::Playlist>(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
	assert_eq!(response.body().songs.paths.len(), 2)
}

#[tokio::test]
async fn get_playlist_bad_name_returns_not_found() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login().await;

	let request = protocol::read_playlist::<V8>(TEST_PLAYLIST_NAME);
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_playlist_requires_auth() {
	let mut service = ServiceType::new(&test_name!()).await;
	let request = protocol::delete_playlist(TEST_PLAYLIST_NAME);
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn delete_playlist_golden_path() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login().await;

	{
		let my_playlist = dto::SavePlaylistInput { tracks: Vec::new() };
		let request = protocol::save_playlist(TEST_PLAYLIST_NAME, my_playlist);
		let response = service.fetch(&request).await;
		assert_eq!(response.status(), StatusCode::OK);
	}

	let request = protocol::delete_playlist(TEST_PLAYLIST_NAME);
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn export_playlists_golden_path() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	{
		let my_playlist = dto::SavePlaylistInput {
			#[rustfmt::skip]
			tracks: vec![
				[TEST_MOUNT_NAME, "Khemmis", "Hunted", "02 - Candlelight.mp3"].iter().collect(),
				[TEST_MOUNT_NAME, "Khemmis", "Hunted", "05 - Hunted.mp3"].iter().collect(),
			],
		};
		let request = protocol::save_playlist(TEST_PLAYLIST_NAME, my_playlist);
		let response = service.fetch(&request).await;
		assert_eq!(response.status(), StatusCode::OK);
	}

	let request = protocol::export_playlists();
	let response = service.fetch_bytes(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
	let content_disposition = response.headers().get(header::CONTENT_DISPOSITION).unwrap();
	assert_eq!(
		content_disposition.to_str().unwrap(),
		r#"attachment; filename="polaris-playlists-test_user.zip""#
	);

	let mut expected = Vec::new();
	File::open("test-data/playlists/export-playlists-golden-path.zip")
		.unwrap()
		.read_to_end(&mut expected)
		.unwrap();
	assert_eq!(&expected, response.body());
}
