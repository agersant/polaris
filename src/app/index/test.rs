use std::path::{Path, PathBuf};

use super::*;
use crate::app::{scanner, test};
use crate::test_name;

const TEST_MOUNT_NAME: &str = "root";

#[tokio::test]
async fn can_browse_top_level() {
	let ctx = test::ContextBuilder::new(test_name!())
		.mount(TEST_MOUNT_NAME, "test-data/small-collection")
		.build()
		.await;
	ctx.scanner.scan().await.unwrap();

	let root_path = Path::new(TEST_MOUNT_NAME);
	let files = ctx.index.browse(Path::new("")).await.unwrap();
	assert_eq!(files.len(), 1);
	match files[0] {
		CollectionFile::Directory(ref d) => assert_eq!(d.path, root_path.to_str().unwrap()),
		_ => panic!("Expected directory"),
	}
}

#[tokio::test]
async fn can_browse_directory() {
	let khemmis_path: PathBuf = [TEST_MOUNT_NAME, "Khemmis"].iter().collect();
	let tobokegao_path: PathBuf = [TEST_MOUNT_NAME, "Tobokegao"].iter().collect();

	let ctx = test::ContextBuilder::new(test_name!())
		.mount(TEST_MOUNT_NAME, "test-data/small-collection")
		.build()
		.await;
	ctx.scanner.scan().await.unwrap();

	let files = ctx.index.browse(Path::new(TEST_MOUNT_NAME)).await.unwrap();

	assert_eq!(files.len(), 2);
	match files[0] {
		CollectionFile::Directory(ref d) => assert_eq!(d.path, khemmis_path.to_str().unwrap()),
		_ => panic!("Expected directory"),
	}

	match files[1] {
		CollectionFile::Directory(ref d) => assert_eq!(d.path, tobokegao_path.to_str().unwrap()),
		_ => panic!("Expected directory"),
	}
}

#[tokio::test]
async fn can_flatten_root() {
	let ctx = test::ContextBuilder::new(test_name!())
		.mount(TEST_MOUNT_NAME, "test-data/small-collection")
		.build()
		.await;
	ctx.scanner.scan().await.unwrap();
	let songs = ctx.index.flatten(Path::new(TEST_MOUNT_NAME)).await.unwrap();
	assert_eq!(songs.len(), 13);
	assert_eq!(songs[0].title, Some("Above The Water".to_owned()));
}

#[tokio::test]
async fn can_flatten_directory() {
	let ctx = test::ContextBuilder::new(test_name!())
		.mount(TEST_MOUNT_NAME, "test-data/small-collection")
		.build()
		.await;
	ctx.scanner.scan().await.unwrap();
	let path: PathBuf = [TEST_MOUNT_NAME, "Tobokegao"].iter().collect();
	let songs = ctx.index.flatten(path).await.unwrap();
	assert_eq!(songs.len(), 8);
}

#[tokio::test]
async fn can_flatten_directory_with_shared_prefix() {
	let ctx = test::ContextBuilder::new(test_name!())
		.mount(TEST_MOUNT_NAME, "test-data/small-collection")
		.build()
		.await;
	ctx.scanner.scan().await.unwrap();
	let path: PathBuf = [TEST_MOUNT_NAME, "Tobokegao", "Picnic"].iter().collect(); // Prefix of '(Picnic Remixes)'
	let songs = ctx.index.flatten(path).await.unwrap();
	assert_eq!(songs.len(), 7);
}

#[tokio::test]
async fn can_get_random_albums() {
	let ctx = test::ContextBuilder::new(test_name!())
		.mount(TEST_MOUNT_NAME, "test-data/small-collection")
		.build()
		.await;
	ctx.scanner.scan().await.unwrap();
	let albums = ctx.index.get_random_albums(1).await.unwrap();
	assert_eq!(albums.len(), 1);
}

#[tokio::test]
async fn can_get_recent_albums() {
	let ctx = test::ContextBuilder::new(test_name!())
		.mount(TEST_MOUNT_NAME, "test-data/small-collection")
		.build()
		.await;
	ctx.scanner.scan().await.unwrap();
	let albums = ctx.index.get_recent_albums(2).await.unwrap();
	assert_eq!(albums.len(), 2);
	assert!(albums[0].date_added >= albums[1].date_added);
}

#[tokio::test]
async fn can_get_a_song() {
	let ctx = test::ContextBuilder::new(test_name!())
		.mount(TEST_MOUNT_NAME, "test-data/small-collection")
		.build()
		.await;

	ctx.scanner.scan().await.unwrap();

	let picnic_virtual_dir: PathBuf = [TEST_MOUNT_NAME, "Tobokegao", "Picnic"].iter().collect();
	let song_virtual_path = picnic_virtual_dir.join("05 - シャーベット (Sherbet).mp3");
	let artwork_virtual_path = picnic_virtual_dir.join("Folder.png");

	let song = ctx.index.get_song(&song_virtual_path).await.unwrap();
	assert_eq!(song.path, song_virtual_path.to_string_lossy().as_ref());
	assert_eq!(song.track_number, Some(5));
	assert_eq!(song.disc_number, None);
	assert_eq!(song.title, Some("シャーベット (Sherbet)".to_owned()));
	assert_eq!(
		song.artists,
		scanner::MultiString(vec!["Tobokegao".to_owned()])
	);
	assert_eq!(song.album_artists, scanner::MultiString(vec![]));
	assert_eq!(song.album, Some("Picnic".to_owned()));
	assert_eq!(song.year, Some(2016));
	assert_eq!(
		song.artwork,
		Some(artwork_virtual_path.to_string_lossy().into_owned())
	);
}
