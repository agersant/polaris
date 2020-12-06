use http::StatusCode;

use crate::index;
use crate::service::dto;
use crate::service::test::{constants::*, protocol, ServiceType, TestService};
use crate::test_name;

#[test]
fn test_list_playlists_requires_auth() {
	let mut service = ServiceType::new(&test_name!());
	let request = protocol::playlists();
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_list_playlists_golden_path() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login();
	let request = protocol::playlists();
	let response = service.fetch_json::<_, Vec<dto::ListPlaylistsEntry>>(&request);
	assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn test_save_playlist_requires_auth() {
	let mut service = ServiceType::new(&test_name!());
	let my_playlist = dto::SavePlaylistInput { tracks: Vec::new() };
	let request = protocol::save_playlist(TEST_PLAYLIST_NAME, my_playlist);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_save_playlist_golden_path() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login();

	let my_playlist = dto::SavePlaylistInput { tracks: Vec::new() };
	let request = protocol::save_playlist(TEST_PLAYLIST_NAME, my_playlist);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn test_get_playlist_requires_auth() {
	let mut service = ServiceType::new(&test_name!());
	let request = protocol::read_playlist(TEST_PLAYLIST_NAME);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_get_playlist_golden_path() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login();

	{
		let my_playlist = dto::SavePlaylistInput { tracks: Vec::new() };
		let request = protocol::save_playlist(TEST_PLAYLIST_NAME, my_playlist);
		let response = service.fetch(&request);
		assert_eq!(response.status(), StatusCode::OK);
	}

	let request = protocol::read_playlist(TEST_PLAYLIST_NAME);
	let response = service.fetch_json::<_, Vec<index::Song>>(&request);
	assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn test_get_playlist_bad_name_returns_not_found() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login();

	let request = protocol::read_playlist(TEST_PLAYLIST_NAME);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn test_delete_playlist_requires_auth() {
	let mut service = ServiceType::new(&test_name!());
	let request = protocol::delete_playlist(TEST_PLAYLIST_NAME);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_delete_playlist_golden_path() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login();

	{
		let my_playlist = dto::SavePlaylistInput { tracks: Vec::new() };
		let request = protocol::save_playlist(TEST_PLAYLIST_NAME, my_playlist);
		let response = service.fetch(&request);
		assert_eq!(response.status(), StatusCode::OK);
	}

	let request = protocol::delete_playlist(TEST_PLAYLIST_NAME);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn test_delete_playlist_bad_name_returns_not_found() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login();

	let request = protocol::delete_playlist(TEST_PLAYLIST_NAME);
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
