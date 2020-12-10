use std::path::{Path, PathBuf};

use super::*;
use crate::app::vfs;
use crate::db::{self, directories, songs};
use crate::test_name;

#[test]
fn test_populate() {
	let db = db::get_test_db(&test_name!());
	let vfs_manager = vfs::Manager::new(db.clone());
	let index = Index::new(db.clone(), vfs_manager);
	index.update().unwrap();
	index.update().unwrap(); // Validates that subsequent updates don't run into conflicts

	let connection = db.connect().unwrap();
	let all_directories: Vec<Directory> = directories::table.load(&connection).unwrap();
	let all_songs: Vec<Song> = songs::table.load(&connection).unwrap();
	assert_eq!(all_directories.len(), 6);
	assert_eq!(all_songs.len(), 13);
}

#[test]
fn test_metadata() {
	let target: PathBuf = ["test-data", "small-collection", "Tobokegao", "Picnic"]
		.iter()
		.collect();

	let mut song_path = target.clone();
	song_path.push("05 - シャーベット (Sherbet).mp3");

	let mut artwork_path = target.clone();
	artwork_path.push("Folder.png");

	let db = db::get_test_db(&test_name!());
	let vfs_manager = vfs::Manager::new(db.clone());
	let index = Index::new(db.clone(), vfs_manager);
	index.update().unwrap();

	let connection = db.connect().unwrap();
	let songs: Vec<Song> = songs::table
		.filter(songs::title.eq("シャーベット (Sherbet)"))
		.load(&connection)
		.unwrap();

	assert_eq!(songs.len(), 1);
	let song = &songs[0];
	assert_eq!(song.path, song_path.to_string_lossy().as_ref());
	assert_eq!(song.track_number, Some(5));
	assert_eq!(song.disc_number, None);
	assert_eq!(song.title, Some("シャーベット (Sherbet)".to_owned()));
	assert_eq!(song.artist, Some("Tobokegao".to_owned()));
	assert_eq!(song.album_artist, None);
	assert_eq!(song.album, Some("Picnic".to_owned()));
	assert_eq!(song.year, Some(2016));
	assert_eq!(
		song.artwork,
		Some(artwork_path.to_string_lossy().into_owned())
	);
}

#[test]
fn test_embedded_artwork() {
	let song_path: PathBuf = [
		"test-data",
		"small-collection",
		"Tobokegao",
		"Picnic",
		"07 - なぜ (Why).mp3",
	]
	.iter()
	.collect();

	let db = db::get_test_db(&test_name!());
	let vfs_manager = vfs::Manager::new(db.clone());
	let index = Index::new(db.clone(), vfs_manager);
	index.update().unwrap();

	let connection = db.connect().unwrap();
	let songs: Vec<Song> = songs::table
		.filter(songs::title.eq("なぜ (Why?)"))
		.load(&connection)
		.unwrap();

	assert_eq!(songs.len(), 1);
	let song = &songs[0];
	assert_eq!(song.artwork, Some(song_path.to_string_lossy().into_owned()));
}

#[test]
fn test_browse_top_level() {
	let mut root_path = PathBuf::new();
	root_path.push("root");

	let db = db::get_test_db(&test_name!());
	let vfs_manager = vfs::Manager::new(db.clone());
	let index = Index::new(db, vfs_manager);
	index.update().unwrap();

	let results = index.browse(Path::new("")).unwrap();

	assert_eq!(results.len(), 1);
	match results[0] {
		CollectionFile::Directory(ref d) => assert_eq!(d.path, root_path.to_str().unwrap()),
		_ => panic!("Expected directory"),
	}
}

#[test]
fn test_browse() {
	let khemmis_path: PathBuf = ["root", "Khemmis"].iter().collect();
	let tobokegao_path: PathBuf = ["root", "Tobokegao"].iter().collect();

	let db = db::get_test_db(&test_name!());
	let vfs_manager = vfs::Manager::new(db.clone());
	let index = Index::new(db, vfs_manager);
	index.update().unwrap();

	let results = index.browse(Path::new("root")).unwrap();

	assert_eq!(results.len(), 2);
	match results[0] {
		CollectionFile::Directory(ref d) => assert_eq!(d.path, khemmis_path.to_str().unwrap()),
		_ => panic!("Expected directory"),
	}
	match results[1] {
		CollectionFile::Directory(ref d) => assert_eq!(d.path, tobokegao_path.to_str().unwrap()),
		_ => panic!("Expected directory"),
	}
}

#[test]
fn test_flatten() {
	let db = db::get_test_db(&test_name!());
	let vfs_manager = vfs::Manager::new(db.clone());
	let index = Index::new(db, vfs_manager);
	index.update().unwrap();

	// Flatten all
	let results = index.flatten(Path::new("root")).unwrap();
	assert_eq!(results.len(), 13);
	assert_eq!(results[0].title, Some("Above The Water".to_owned()));

	// Flatten a directory
	let path: PathBuf = ["root", "Tobokegao"].iter().collect();
	let results = index.flatten(&path).unwrap();
	assert_eq!(results.len(), 8);

	// Flatten a directory that is a prefix of another directory (Picnic Remixes)
	let path: PathBuf = ["root", "Tobokegao", "Picnic"].iter().collect();
	let results = index.flatten(&path).unwrap();
	assert_eq!(results.len(), 7);
}

#[test]
fn test_random() {
	let db = db::get_test_db(&test_name!());
	let vfs_manager = vfs::Manager::new(db.clone());
	let index = Index::new(db, vfs_manager);
	index.update().unwrap();

	let results = index.get_random_albums(1).unwrap();
	assert_eq!(results.len(), 1);
}

#[test]
fn test_recent() {
	let db = db::get_test_db(&test_name!());
	let vfs_manager = vfs::Manager::new(db.clone());
	let index = Index::new(db, vfs_manager);
	index.update().unwrap();

	let results = index.get_recent_albums(2).unwrap();
	assert_eq!(results.len(), 2);
	assert!(results[0].date_added >= results[1].date_added);
}

#[test]
fn test_get_song() {
	let db = db::get_test_db(&test_name!());
	let vfs_manager = vfs::Manager::new(db.clone());
	let index = Index::new(db, vfs_manager);
	index.update().unwrap();

	let song_path: PathBuf = ["root", "Khemmis", "Hunted", "02 - Candlelight.mp3"]
		.iter()
		.collect();

	let song = index.get_song(&song_path).unwrap();
	assert_eq!(song.title.unwrap(), "Candlelight");
}
