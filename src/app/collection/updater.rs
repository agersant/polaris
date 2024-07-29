use std::{sync::Arc, time::Duration};

use log::{error, info};
use tokio::{
	sync::{mpsc::unbounded_channel, Notify},
	time::Instant,
};

use crate::{
	app::{collection::*, settings, vfs},
	db::DB,
};

#[derive(Clone)]
pub struct Updater {
	db: DB,
	index: Index,
	settings_manager: settings::Manager,
	vfs_manager: vfs::Manager,
	pending_scan: Arc<Notify>,
}

impl Updater {
	pub fn new(
		db: DB,
		index: Index,
		settings_manager: settings::Manager,
		vfs_manager: vfs::Manager,
	) -> Self {
		let updater = Self {
			db,
			index,
			vfs_manager,
			settings_manager,
			pending_scan: Arc::new(Notify::new()),
		};

		tokio::spawn({
			let mut updater = updater.clone();
			async move {
				loop {
					updater.pending_scan.notified().await;
					if let Err(e) = updater.update().await {
						error!("Error while updating index: {}", e);
					}
				}
			}
		});

		updater
	}

	pub fn trigger_scan(&self) {
		self.pending_scan.notify_one();
	}

	pub fn begin_periodic_scans(&self) {
		tokio::spawn({
			let index = self.clone();
			async move {
				loop {
					index.trigger_scan();
					let sleep_duration = index
						.settings_manager
						.get_index_sleep_duration()
						.await
						.unwrap_or_else(|e| {
							error!("Could not retrieve index sleep duration: {}", e);
							Duration::from_secs(1800)
						});
					tokio::time::sleep(sleep_duration).await;
				}
			}
		});
	}

	pub async fn update(&mut self) -> Result<(), Error> {
		let start = Instant::now();
		info!("Beginning library index update");

		let cleaner = Cleaner::new(self.db.clone(), self.vfs_manager.clone());
		cleaner.clean().await?;

		let album_art_pattern = self
			.settings_manager
			.get_index_album_art_pattern()
			.await
			.ok();

		let (scanner_directories_output, mut collection_directories_input) = unbounded_channel();
		let (scanner_songs_output, mut collection_songs_input) = unbounded_channel();

		let scanner = Scanner::new(
			scanner_directories_output,
			scanner_songs_output,
			self.vfs_manager.clone(),
			album_art_pattern,
		);

		let mut song_inserter = Inserter::<Song>::new(self.db.clone());
		let mut directory_inserter = Inserter::<Directory>::new(self.db.clone());

		let directory_task = tokio::spawn(async move {
			let capacity = 500;
			let mut buffer: Vec<Directory> = Vec::with_capacity(capacity);
			loop {
				match collection_directories_input
					.recv_many(&mut buffer, capacity)
					.await
				{
					0 => break,
					_ => {
						for directory in buffer.drain(0..) {
							directory_inserter.insert(directory).await;
						}
					}
				}
			}
			directory_inserter.flush().await;
		});

		let song_task = tokio::spawn(async move {
			let capacity = 500;
			let mut lookup_tables = Lookups::default();
			let mut buffer: Vec<Song> = Vec::with_capacity(capacity);

			loop {
				match collection_songs_input
					.recv_many(&mut buffer, capacity)
					.await
				{
					0 => break,
					_ => {
						for song in buffer.drain(0..) {
							lookup_tables.add_song(&song);
							song_inserter.insert(song).await;
						}
					}
				}
			}
			song_inserter.flush().await;
			lookup_tables
		});

		let lookup_tables = tokio::join!(scanner.scan(), directory_task, song_task).2?;
		self.index.replace_lookup_tables(lookup_tables).await;

		info!(
			"Library index update took {} seconds",
			start.elapsed().as_millis() as f32 / 1000.0
		);

		Ok(())
	}
}

#[cfg(test)]
mod test {
	use std::path::PathBuf;

	use crate::{
		app::{collection::*, settings, test},
		test_name,
	};

	const TEST_MOUNT_NAME: &str = "root";

	#[tokio::test]
	async fn scan_adds_new_content() {
		let mut ctx = test::ContextBuilder::new(test_name!())
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build()
			.await;

		ctx.updater.update().await.unwrap();
		ctx.updater.update().await.unwrap(); // Validates that subsequent updates don't run into conflicts

		let mut connection = ctx.db.connect().await.unwrap();
		let all_directories = sqlx::query_as!(Directory, "SELECT * FROM directories")
			.fetch_all(connection.as_mut())
			.await
			.unwrap();
		let all_songs = sqlx::query_as!(Song, "SELECT * FROM songs")
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

		let mut ctx = builder
			.mount(TEST_MOUNT_NAME, test_collection_dir.to_str().unwrap())
			.build()
			.await;

		ctx.updater.update().await.unwrap();

		{
			let mut connection = ctx.db.connect().await.unwrap();
			let all_directories = sqlx::query_as!(Directory, "SELECT * FROM directories")
				.fetch_all(connection.as_mut())
				.await
				.unwrap();
			let all_songs = sqlx::query_as!(Song, "SELECT * FROM songs")
				.fetch_all(connection.as_mut())
				.await
				.unwrap();
			assert_eq!(all_directories.len(), 6);
			assert_eq!(all_songs.len(), 13);
		}

		let khemmis_directory = test_collection_dir.join("Khemmis");
		std::fs::remove_dir_all(khemmis_directory).unwrap();
		ctx.updater.update().await.unwrap();
		{
			let mut connection = ctx.db.connect().await.unwrap();
			let all_directories = sqlx::query_as!(Directory, "SELECT * FROM directories")
				.fetch_all(connection.as_mut())
				.await
				.unwrap();
			let all_songs = sqlx::query_as!(Song, "SELECT * FROM songs")
				.fetch_all(connection.as_mut())
				.await
				.unwrap();
			assert_eq!(all_directories.len(), 4);
			assert_eq!(all_songs.len(), 8);
		}
	}

	#[tokio::test]
	async fn finds_embedded_artwork() {
		let mut ctx = test::ContextBuilder::new(test_name!())
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build()
			.await;

		ctx.updater.update().await.unwrap();

		let picnic_virtual_dir: PathBuf = [TEST_MOUNT_NAME, "Tobokegao", "Picnic"].iter().collect();
		let song_virtual_path = picnic_virtual_dir.join("07 - なぜ (Why).mp3");

		let song = ctx.browser.get_song(&song_virtual_path).await.unwrap();
		assert_eq!(
			song.artwork,
			Some(song_virtual_path.to_string_lossy().into_owned())
		);
	}

	#[tokio::test]
	async fn album_art_pattern_is_case_insensitive() {
		let mut ctx = test::ContextBuilder::new(test_name!())
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
			ctx.updater.update().await.unwrap();

			let hunted_virtual_dir: PathBuf =
				[TEST_MOUNT_NAME, "Khemmis", "Hunted"].iter().collect();
			let artwork_virtual_path = hunted_virtual_dir.join("Folder.jpg");
			let song = &ctx.browser.flatten(&hunted_virtual_dir).await.unwrap()[0];
			assert_eq!(
				song.artwork,
				Some(artwork_virtual_path.to_string_lossy().into_owned())
			);
		}
	}
}
