use core::clone::Clone;
use std::path::{Path, PathBuf};

use super::*;
use crate::db;

use crate::test_name;

#[test]
fn test_create_playlist() {
	let db = db::get_test_db(&test_name!());
	let manager = Manager::new(db);

	let found_playlists = manager.list_playlists("test_user").unwrap();
	assert!(found_playlists.is_empty());

	manager
		.save_playlist("chill_and_grill", "test_user", &Vec::new())
		.unwrap();
	let found_playlists = manager.list_playlists("test_user").unwrap();
	assert_eq!(found_playlists.len(), 1);
	assert_eq!(found_playlists[0], "chill_and_grill");

	let found_playlists = manager.list_playlists("someone_else");
	assert!(found_playlists.is_err());
}

#[test]
fn test_delete_playlist() {
	let db = db::get_test_db(&test_name!());
	let manager = Manager::new(db);
	let playlist_content = Vec::new();

	manager
		.save_playlist("chill_and_grill", "test_user", &playlist_content)
		.unwrap();
	manager
		.save_playlist("mellow_bungalow", "test_user", &playlist_content)
		.unwrap();
	let found_playlists = manager.list_playlists("test_user").unwrap();
	assert_eq!(found_playlists.len(), 2);

	manager
		.delete_playlist("chill_and_grill", "test_user")
		.unwrap();
	let found_playlists = manager.list_playlists("test_user").unwrap();
	assert_eq!(found_playlists.len(), 1);
	assert_eq!(found_playlists[0], "mellow_bungalow");

	let delete_result = manager.delete_playlist("mellow_bungalow", "someone_else");
	assert!(delete_result.is_err());
}

#[test]
fn test_fill_playlist() {
	use crate::index;

	let db = db::get_test_db(&test_name!());
	let manager = Manager::new(db.clone());

	index::update(&db).unwrap();

	let mut playlist_content: Vec<String> = index::flatten(&db, Path::new("root"))
		.unwrap()
		.into_iter()
		.map(|s| s.path)
		.collect();
	assert_eq!(playlist_content.len(), 13);

	let first_song = playlist_content[0].clone();
	playlist_content.push(first_song);
	assert_eq!(playlist_content.len(), 14);

	manager
		.save_playlist("all_the_music", "test_user", &playlist_content)
		.unwrap();

	let songs = manager.read_playlist("all_the_music", "test_user").unwrap();
	assert_eq!(songs.len(), 14);
	assert_eq!(songs[0].title, Some("Above The Water".to_owned()));
	assert_eq!(songs[13].title, Some("Above The Water".to_owned()));

	let first_song_path: PathBuf = ["root", "Khemmis", "Hunted", "01 - Above The Water.mp3"]
		.iter()
		.collect();
	assert_eq!(songs[0].path, first_song_path.to_str().unwrap());

	// Save again to verify that we don't dupe the content
	manager
		.save_playlist("all_the_music", "test_user", &playlist_content)
		.unwrap();
	let songs = manager.read_playlist("all_the_music", "test_user").unwrap();
	assert_eq!(songs.len(), 14);
}
