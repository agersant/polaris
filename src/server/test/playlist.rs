use std::path::Path;

use http::StatusCode;

use crate::server::dto::{self};
use crate::server::test::protocol::{V7, V8};
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
	service.login().await;

	{
		let my_playlist = dto::SavePlaylistInput { tracks: Vec::new() };
		let request = protocol::save_playlist(TEST_PLAYLIST_NAME, my_playlist);
		let response = service.fetch(&request).await;
		assert_eq!(response.status(), StatusCode::OK);
	}

	let request = protocol::read_playlist::<V8>(TEST_PLAYLIST_NAME);
	let response = service.fetch_json::<_, dto::Playlist>(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn get_playlist_golden_path_api_v7() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login().await;

	{
		let my_playlist = dto::SavePlaylistInput { tracks: Vec::new() };
		let request = protocol::save_playlist(TEST_PLAYLIST_NAME, my_playlist);
		let response = service.fetch(&request).await;
		assert_eq!(response.status(), StatusCode::OK);
	}

	let request = protocol::read_playlist::<V7>(TEST_PLAYLIST_NAME);
	let response = service.fetch_json::<_, Vec<dto::v7::Song>>(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
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
