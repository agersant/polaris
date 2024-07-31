use std::{sync::Arc, time::Duration};

use log::{error, info};
use tokio::{
	sync::{mpsc::unbounded_channel, Notify},
	time::Instant,
};

use crate::app::{collection::*, settings, vfs};

#[derive(Clone)]
pub struct Updater {
	index_manager: IndexManager,
	settings_manager: settings::Manager,
	vfs_manager: vfs::Manager,
	pending_scan: Arc<Notify>,
}

impl Updater {
	pub async fn new(
		index_manager: IndexManager,
		settings_manager: settings::Manager,
		vfs_manager: vfs::Manager,
	) -> Result<Self, Error> {
		let updater = Self {
			index_manager,
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

		Ok(updater)
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
		info!("Beginning collection scan");

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

		let index_task = tokio::spawn(async move {
			let capacity = 500;
			let mut index_builder = IndexBuilder::default();
			let mut song_buffer: Vec<Song> = Vec::with_capacity(capacity);
			let mut directory_buffer: Vec<Directory> = Vec::with_capacity(capacity);

			loop {
				let exhausted_songs = match collection_songs_input
					.recv_many(&mut song_buffer, capacity)
					.await
				{
					0 => true,
					_ => {
						for song in song_buffer.drain(0..) {
							index_builder.add_song(song);
						}
						false
					}
				};

				let exhausted_directories = match collection_directories_input
					.recv_many(&mut directory_buffer, capacity)
					.await
				{
					0 => true,
					_ => {
						for directory in directory_buffer.drain(0..) {
							index_builder.add_directory(directory);
						}
						false
					}
				};

				if exhausted_directories && exhausted_songs {
					break;
				}
			}

			index_builder.build()
		});

		let index = tokio::join!(scanner.scan(), index_task).1?;
		self.index_manager.persist_index(&index).await?;
		self.index_manager.replace_index(index).await;

		info!(
			"Collection scan took {} seconds",
			start.elapsed().as_millis() as f32 / 1000.0
		);

		Ok(())
	}
}

#[cfg(test)]
mod test {
	use std::path::PathBuf;

	use crate::{
		app::{settings, test},
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

		todo!();

		// assert_eq!(all_directories.len(), 6);
		// assert_eq!(all_songs.len(), 13);
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
		assert_eq!(song.artwork, Some(song_virtual_path));
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
			assert_eq!(song.artwork, Some(artwork_virtual_path));
		}
	}
}
