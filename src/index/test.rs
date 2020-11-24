use std::path::{Path, PathBuf};

use crate::db;
use crate::db::{directories, songs};
use crate::index::*;

#[test]
fn test_populate() {
	let db = db::get_test_db("populate.sqlite");
	update(&db).unwrap();
	update(&db).unwrap(); // Check that subsequent updates don't run into conflicts

	let connection = db.connect().unwrap();
	let all_directories: Vec<Directory> = directories::table.load(&connection).unwrap();
	let all_songs: Vec<Song> = songs::table.load(&connection).unwrap();
	assert_eq!(all_directories.len(), 6);
	assert_eq!(all_songs.len(), 13);
}

#[test]
fn test_metadata() {
	let mut target = PathBuf::new();
	target.push("test-data");
	target.push("small-collection");
	target.push("Tobokegao");
	target.push("Picnic");

	let mut song_path = target.clone();
	song_path.push("05 - シャーベット (Sherbet).mp3");

	let mut artwork_path = target.clone();
	artwork_path.push("Folder.png");

	let db = db::get_test_db("metadata.sqlite");
	update(&db).unwrap();

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
	let mut song_path = PathBuf::new();
	song_path.push("test-data");
	song_path.push("small-collection");
	song_path.push("Tobokegao");
	song_path.push("Picnic");
	song_path.push("07 - なぜ (Why).mp3");

	let db = db::get_test_db("artwork.sqlite");
	update(&db).unwrap();

	let connection = db.connect().unwrap();
	let songs: Vec<Song> = songs::table
		.filter(songs::title.eq("なぜ (Why?)"))
		.load(&connection)
		.unwrap();

	assert_eq!(songs.len(), 1);
	let song = &songs[0];
	assert_eq!(song.path, song_path.to_string_lossy().as_ref());
	assert_eq!(song.track_number, Some(7));
	assert_eq!(song.disc_number, None);
	assert_eq!(song.title, Some("なぜ (Why?)".to_owned()));
	assert_eq!(song.artist, Some("Tobokegao".to_owned()));
	assert_eq!(song.album_artist, None);
	assert_eq!(song.album, Some("Picnic".to_owned()));
	assert_eq!(song.year, Some(2016));
	assert_eq!(song.artwork, Some(song_path.to_string_lossy().into_owned()));
}

#[test]
fn test_browse_top_level() {
	let mut root_path = PathBuf::new();
	root_path.push("root");

	let db = db::get_test_db("browse_top_level.sqlite");
	update(&db).unwrap();
	let results = browse(&db, Path::new("")).unwrap();

	assert_eq!(results.len(), 1);
	match results[0] {
		CollectionFile::Directory(ref d) => assert_eq!(d.path, root_path.to_str().unwrap()),
		_ => panic!("Expected directory"),
	}
}

#[test]
fn test_browse() {
	let mut khemmis_path = PathBuf::new();
	khemmis_path.push("root");
	khemmis_path.push("Khemmis");

	let mut tobokegao_path = PathBuf::new();
	tobokegao_path.push("root");
	tobokegao_path.push("Tobokegao");

	let db = db::get_test_db("browse.sqlite");
	update(&db).unwrap();
	let results = browse(&db, Path::new("root")).unwrap();

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
	let db = db::get_test_db("flatten.sqlite");
	update(&db).unwrap();

	// Flatten all
	let results = flatten(&db, Path::new("root")).unwrap();
	assert_eq!(results.len(), 13);
	assert_eq!(results[0].title, Some("Above The Water".to_owned()));

	// Flatten a directory
	let mut path = PathBuf::new();
	path.push("root");
	path.push("Tobokegao");
	let results = flatten(&db, &path).unwrap();
	assert_eq!(results.len(), 8);

	// Flatten a directory that is a prefix of another directory (Picnic Remixes)
	let mut path = PathBuf::new();
	path.push("root");
	path.push("Tobokegao");
	path.push("Picnic");
	let results = flatten(&db, &path).unwrap();
	assert_eq!(results.len(), 7);
}

#[test]
fn test_random() {
	let db = db::get_test_db("random.sqlite");
	update(&db).unwrap();
	let results = get_random_albums(&db, 1).unwrap();
	assert_eq!(results.len(), 1);
}

#[test]
fn test_recent() {
	let db = db::get_test_db("recent.sqlite");
	update(&db).unwrap();
	let results = get_recent_albums(&db, 2).unwrap();
	assert_eq!(results.len(), 2);
	assert!(results[0].date_added >= results[1].date_added);
}

#[test]
fn test_get_song() {
	let db = db::get_test_db("get_song.sqlite");
	update(&db).unwrap();

	let mut song_path = PathBuf::new();
	song_path.push("root");
	song_path.push("Khemmis");
	song_path.push("Hunted");
	song_path.push("02 - Candlelight.mp3");

	let song = get_song(&db, &song_path).unwrap();
	assert_eq!(song.title.unwrap(), "Candlelight");
}
