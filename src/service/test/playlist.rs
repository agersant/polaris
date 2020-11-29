use http::StatusCode;
use std::path::PathBuf;

use crate::index;
use crate::service::dto;
use crate::service::test::{constants::*, ServiceType, TestService};

#[test]
fn test_service_playlists() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();
	service.index();

	let list_playlists = service.request_builder().playlists();

	// List some songs
	let playlist_name = "my_playlist";
	let my_songs = {
		let request = service.request_builder().flatten(&PathBuf::new());
		let response = service.fetch_json::<_, Vec<index::Song>>(&request);
		assert_eq!(response.status(), StatusCode::OK);
		let mut my_songs = response.into_body();
		my_songs.pop();
		my_songs.pop();
		my_songs
	};

	// Verify no existing playlists
	{
		let response = service.fetch_json::<_, Vec<dto::ListPlaylistsEntry>>(&list_playlists);
		assert_eq!(response.status(), StatusCode::OK);
		let playlists = response.body();
		assert_eq!(playlists.len(), 0);
	}

	// Store a playlist
	{
		let my_playlist = dto::SavePlaylistInput {
			tracks: my_songs.iter().map(|s| s.path.clone()).collect(),
		};
		let request = service
			.request_builder()
			.save_playlist(playlist_name, my_playlist);
		let response = service.fetch(&request);
		assert_eq!(response.status(), StatusCode::OK);
	}

	// Verify new playlist is listed
	{
		let response = service.fetch_json::<_, Vec<dto::ListPlaylistsEntry>>(&list_playlists);
		assert_eq!(response.status(), StatusCode::OK);
		let playlists = response.body();
		assert_eq!(
			playlists,
			&vec![dto::ListPlaylistsEntry {
				name: playlist_name.to_owned()
			}]
		);
	}

	// Verify content of new playlist
	{
		let request = service.request_builder().read_playlist(playlist_name);
		let response = service.fetch_json::<_, Vec<index::Song>>(&request);
		assert_eq!(response.status(), StatusCode::OK);
		let songs = response.body();
		assert_eq!(songs, &my_songs);
	}

	// Delete playlist
	{
		let request = service.request_builder().delete_playlist(playlist_name);
		let response = service.fetch(&request);
		assert_eq!(response.status(), StatusCode::OK);
	}

	// Verify updated listing
	{
		let response = service.fetch_json::<_, Vec<dto::ListPlaylistsEntry>>(&list_playlists);
		let playlists = response.body();
		assert_eq!(playlists.len(), 0);
	}
}
