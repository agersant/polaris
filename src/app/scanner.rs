use log::{error, info};
use rayon::{Scope, ThreadPoolBuilder};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::{cmp::min, time::Duration};
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::Notify;
use tokio::time::Instant;

use crate::app::{formats, index, settings, vfs, Error};

#[derive(Debug, PartialEq, Eq)]
pub struct Directory {
	pub virtual_path: PathBuf,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Song {
	pub path: PathBuf,
	pub virtual_path: PathBuf,
	pub virtual_parent: PathBuf,
	pub track_number: Option<i64>,
	pub disc_number: Option<i64>,
	pub title: Option<String>,
	pub artists: Vec<String>,
	pub album_artists: Vec<String>,
	pub year: Option<i64>,
	pub album: Option<String>,
	pub artwork: Option<PathBuf>,
	pub duration: Option<i64>,
	pub lyricists: Vec<String>,
	pub composers: Vec<String>,
	pub genres: Vec<String>,
	pub labels: Vec<String>,
	pub date_added: i64,
}

#[derive(Clone)]
pub struct Scanner {
	index_manager: index::Manager,
	settings_manager: settings::Manager,
	vfs_manager: vfs::Manager,
	pending_scan: Arc<Notify>,
}

impl Scanner {
	pub async fn new(
		index_manager: index::Manager,
		settings_manager: settings::Manager,
		vfs_manager: vfs::Manager,
	) -> Result<Self, Error> {
		let scanner = Self {
			index_manager,
			vfs_manager,
			settings_manager,
			pending_scan: Arc::new(Notify::new()),
		};

		tokio::spawn({
			let mut scanner = scanner.clone();
			async move {
				loop {
					scanner.pending_scan.notified().await;
					if let Err(e) = scanner.update().await {
						error!("Error while updating index: {}", e);
					}
				}
			}
		});

		Ok(scanner)
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

		let (scan_directories_output, mut collection_directories_input) = unbounded_channel();
		let (scan_songs_output, mut collection_songs_input) = unbounded_channel();

		let scan = Scan::new(
			scan_directories_output,
			scan_songs_output,
			self.vfs_manager.clone(),
			album_art_pattern,
		);

		let index_task = tokio::task::spawn_blocking(move || {
			let mut index_builder = index::Builder::default();

			loop {
				let exhausted_songs = match collection_songs_input.try_recv() {
					Ok(song) => {
						index_builder.add_song(song);
						false
					}
					Err(TryRecvError::Empty) => {
						std::thread::sleep(Duration::from_millis(1));
						false
					}
					Err(TryRecvError::Disconnected) => true,
				};

				let exhausted_directories = match collection_directories_input.try_recv() {
					Ok(directory) => {
						index_builder.add_directory(directory);
						false
					}
					Err(TryRecvError::Empty) => false,
					Err(TryRecvError::Disconnected) => true,
				};

				if exhausted_directories && exhausted_songs {
					break;
				}
			}

			index_builder.build()
		});

		let index = tokio::join!(scan.start(), index_task).1?;
		self.index_manager.persist_index(&index).await?;
		self.index_manager.replace_index(index).await;

		info!(
			"Collection scan took {} seconds",
			start.elapsed().as_millis() as f32 / 1000.0
		);

		Ok(())
	}
}

struct Scan {
	directories_output: UnboundedSender<Directory>,
	songs_output: UnboundedSender<Song>,
	vfs_manager: vfs::Manager,
	artwork_regex: Option<Regex>,
}

impl Scan {
	pub fn new(
		directories_output: UnboundedSender<Directory>,
		songs_output: UnboundedSender<Song>,
		vfs_manager: vfs::Manager,
		artwork_regex: Option<Regex>,
	) -> Self {
		Self {
			directories_output,
			songs_output,
			vfs_manager,
			artwork_regex,
		}
	}

	pub async fn start(self) -> Result<(), Error> {
		let vfs = self.vfs_manager.get_vfs().await?;
		let roots = vfs.mounts().clone();

		let key = "POLARIS_NUM_TRAVERSER_THREADS";
		let num_threads = std::env::var_os(key)
			.map(|v| v.to_string_lossy().to_string())
			.and_then(|v| usize::from_str(&v).ok())
			.unwrap_or_else(|| min(num_cpus::get(), 8));
		info!("Browsing collection using {} threads", num_threads);

		let directories_output = self.directories_output.clone();
		let songs_output = self.songs_output.clone();
		let artwork_regex = self.artwork_regex.clone();

		let thread_pool = ThreadPoolBuilder::new().num_threads(num_threads).build()?;
		thread_pool.scope({
			|scope| {
				for root in roots {
					scope.spawn(|scope| {
						process_directory(
							scope,
							root.source,
							root.name,
							directories_output.clone(),
							songs_output.clone(),
							artwork_regex.clone(),
						);
					});
				}
			}
		});

		Ok(())
	}
}

fn process_directory<P: AsRef<Path>, Q: AsRef<Path>>(
	scope: &Scope,
	real_path: P,
	virtual_path: Q,
	directories_output: UnboundedSender<Directory>,
	songs_output: UnboundedSender<Song>,
	artwork_regex: Option<Regex>,
) {
	let read_dir = match fs::read_dir(&real_path) {
		Ok(read_dir) => read_dir,
		Err(e) => {
			error!(
				"Directory read error for `{}`: {}",
				real_path.as_ref().display(),
				e
			);
			return;
		}
	};

	let mut songs = vec![];
	let mut artwork_file = None;

	for entry in read_dir {
		let name = match entry {
			Ok(ref f) => f.file_name(),
			Err(e) => {
				error!(
					"File read error within `{}`: {}",
					real_path.as_ref().display(),
					e
				);
				break;
			}
		};

		let entry_real_path = real_path.as_ref().join(&name);
		let entry_virtual_path = virtual_path.as_ref().join(&name);

		if entry_real_path.is_dir() {
			scope.spawn({
				let directories_output = directories_output.clone();
				let songs_output = songs_output.clone();
				let artwork_regex = artwork_regex.clone();
				|scope| {
					process_directory(
						scope,
						entry_real_path,
						entry_virtual_path,
						directories_output,
						songs_output,
						artwork_regex,
					);
				}
			});
		} else if let Some(metadata) = formats::read_metadata(&entry_real_path) {
			songs.push(Song {
				path: entry_real_path.clone(),
				virtual_path: entry_virtual_path.clone(),
				virtual_parent: entry_virtual_path.parent().unwrap().to_owned(),
				track_number: metadata.track_number.map(|n| n as i64),
				disc_number: metadata.disc_number.map(|n| n as i64),
				title: metadata.title,
				artists: metadata.artists,
				album_artists: metadata.album_artists,
				year: metadata.year.map(|n| n as i64),
				album: metadata.album,
				artwork: metadata.has_artwork.then(|| entry_virtual_path.clone()),
				duration: metadata.duration.map(|n| n as i64),
				lyricists: metadata.lyricists,
				composers: metadata.composers,
				genres: metadata.genres,
				labels: metadata.labels,
				date_added: get_date_created(&entry_real_path).unwrap_or_default(),
			});
		} else if artwork_file.is_none()
			&& artwork_regex
				.as_ref()
				.is_some_and(|r| r.is_match(name.to_str().unwrap_or_default()))
		{
			artwork_file = Some(entry_virtual_path);
		}
	}

	for mut song in songs {
		song.artwork = song.artwork.or_else(|| artwork_file.clone());
		songs_output.send(song).ok();
	}

	directories_output
		.send(Directory {
			virtual_path: virtual_path.as_ref().to_owned(),
		})
		.ok();
}

fn get_date_created<P: AsRef<Path>>(path: P) -> Option<i64> {
	if let Ok(t) = fs::metadata(path).and_then(|m| m.created().or_else(|_| m.modified())) {
		t.duration_since(std::time::UNIX_EPOCH)
			.map(|d| d.as_secs() as i64)
			.ok()
	} else {
		None
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

		ctx.scanner.update().await.unwrap();
		ctx.scanner.update().await.unwrap(); // Validates that subsequent updates don't run into conflicts

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

		ctx.scanner.update().await.unwrap();

		let picnic_virtual_dir: PathBuf = [TEST_MOUNT_NAME, "Tobokegao", "Picnic"].iter().collect();
		let song_virtual_path = picnic_virtual_dir.join("07 - なぜ (Why).mp3");

		let song = ctx
			.index_manager
			.get_song(song_virtual_path.clone())
			.await
			.unwrap();
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
			ctx.scanner.update().await.unwrap();

			let hunted_virtual_dir: PathBuf =
				[TEST_MOUNT_NAME, "Khemmis", "Hunted"].iter().collect();
			let artwork_virtual_path = hunted_virtual_dir.join("Folder.jpg");
			let song = &ctx.index_manager.flatten(hunted_virtual_dir).await.unwrap()[0];
			todo!();
			// assert_eq!(song.artwork, Some(artwork_virtual_path));
		}
	}
}
