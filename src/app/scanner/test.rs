use std::path::PathBuf;

use crate::{
	app::{scanner, settings, test},
	test_name,
};

const TEST_MOUNT_NAME: &str = "root";

#[tokio::test]
async fn scan_adds_new_content() {
	let ctx = test::ContextBuilder::new(test_name!())
		.mount(TEST_MOUNT_NAME, "test-data/small-collection")
		.build()
		.await;

	ctx.scanner.scan().await.unwrap();
	ctx.scanner.scan().await.unwrap(); // Validates that subsequent updates don't run into conflicts

	let mut connection = ctx.db.connect().await.unwrap();
	let all_directories = sqlx::query_as!(scanner::Directory, "SELECT * FROM directories")
		.fetch_all(connection.as_mut())
		.await
		.unwrap();
	let all_songs = sqlx::query_as!(scanner::Song, "SELECT * FROM songs")
		.fetch_all(connection.as_mut())
		.await
		.unwrap();
	assert_eq!(all_directories.len(), 6);
	assert_eq!(all_songs.len(), 13);
}

#[tokio::test]
async fn scan_removes_missing_content() {
	let builder = test::ContextBuilder::new(test_name!());

	let original_collection_dir: PathBuf = ["test-data", "small-collection"].iter().collect();
	let test_collection_dir: PathBuf = builder.test_directory.join("small-collection");

	let copy_options = fs_extra::dir::CopyOptions::new();
	fs_extra::dir::copy(
		original_collection_dir,
		&builder.test_directory,
		&copy_options,
	)
	.unwrap();

	let ctx = builder
		.mount(TEST_MOUNT_NAME, test_collection_dir.to_str().unwrap())
		.build()
		.await;

	ctx.scanner.scan().await.unwrap();

	{
		let mut connection = ctx.db.connect().await.unwrap();
		let all_directories = sqlx::query_as!(scanner::Directory, "SELECT * FROM directories")
			.fetch_all(connection.as_mut())
			.await
			.unwrap();
		let all_songs = sqlx::query_as!(scanner::Song, "SELECT * FROM songs")
			.fetch_all(connection.as_mut())
			.await
			.unwrap();
		assert_eq!(all_directories.len(), 6);
		assert_eq!(all_songs.len(), 13);
	}

	let khemmis_directory = test_collection_dir.join("Khemmis");
	std::fs::remove_dir_all(khemmis_directory).unwrap();
	ctx.scanner.scan().await.unwrap();
	{
		let mut connection = ctx.db.connect().await.unwrap();
		let all_directories = sqlx::query_as!(scanner::Directory, "SELECT * FROM directories")
			.fetch_all(connection.as_mut())
			.await
			.unwrap();
		let all_songs = sqlx::query_as!(scanner::Song, "SELECT * FROM songs")
			.fetch_all(connection.as_mut())
			.await
			.unwrap();
		assert_eq!(all_directories.len(), 4);
		assert_eq!(all_songs.len(), 8);
	}
}

#[tokio::test]
async fn finds_embedded_artwork() {
	let ctx = test::ContextBuilder::new(test_name!())
		.mount(TEST_MOUNT_NAME, "test-data/small-collection")
		.build()
		.await;

	ctx.scanner.scan().await.unwrap();

	let picnic_virtual_dir: PathBuf = [TEST_MOUNT_NAME, "Tobokegao", "Picnic"].iter().collect();
	let song_virtual_path = picnic_virtual_dir.join("07 - なぜ (Why).mp3");

	let song = ctx.index.get_song(&song_virtual_path).await.unwrap();
	assert_eq!(
		song.artwork,
		Some(song_virtual_path.to_string_lossy().into_owned())
	);
}

#[tokio::test]
async fn album_art_pattern_is_case_insensitive() {
	let ctx = test::ContextBuilder::new(test_name!())
		.mount(TEST_MOUNT_NAME, "test-data/small-collection")
		.build()
		.await;

	let patterns = vec!["folder", "FOLDER"];

	for pattern in patterns.into_iter() {
		ctx.settings_manager
			.amend(&settings::NewSettings {
				album_art_pattern: Some(pattern.to_owned()),
				..Default::default()
			})
			.await
			.unwrap();
		ctx.scanner.scan().await.unwrap();

		let hunted_virtual_dir: PathBuf = [TEST_MOUNT_NAME, "Khemmis", "Hunted"].iter().collect();
		let artwork_virtual_path = hunted_virtual_dir.join("Folder.jpg");
		let song = &ctx.index.flatten(&hunted_virtual_dir).await.unwrap()[0];
		assert_eq!(
			song.artwork,
			Some(artwork_virtual_path.to_string_lossy().into_owned())
		);
	}
}
