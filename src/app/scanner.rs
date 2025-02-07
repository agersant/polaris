use log::{error, info};
use notify::{RecommendedWatcher, Watcher};
use notify_debouncer_full::{Debouncer, FileIdMap};
use rayon::{Scope, ThreadPoolBuilder};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::mpsc::{channel, Sender, TryRecvError};
use std::sync::Arc;
use std::time::SystemTime;
use std::{cmp::min, time::Duration};
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::{Notify, RwLock};
use tokio::task::JoinSet;
use tokio::time::Instant;

use crate::app::{config, formats, index, Error};

#[derive(Debug, PartialEq, Eq)]
pub struct Directory {
	pub virtual_path: PathBuf,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Song {
	pub real_path: PathBuf,
	pub virtual_path: PathBuf,
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

#[derive(Clone, Default)]
pub enum State {
	#[default]
	Initial,
	Pending,
	InProgress,
	UpToDate,
}

#[derive(Clone)]
struct Parameters {
	artwork_regex: Option<Regex>,
	mount_dirs: Vec<config::MountDir>,
}

impl PartialEq for Parameters {
	fn eq(&self, other: &Self) -> bool {
		self.artwork_regex.as_ref().map(|r| r.as_str())
			== other.artwork_regex.as_ref().map(|r| r.as_str())
			&& self.mount_dirs == other.mount_dirs
	}
}

#[derive(Clone, Default)]
pub struct Status {
	pub state: State,
	pub last_start_time: Option<SystemTime>,
	pub last_end_time: Option<SystemTime>,
	pub num_songs_indexed: u32,
}

#[derive(Clone)]
pub struct Scanner {
	index_manager: index::Manager,
	config_manager: config::Manager,
	file_watcher: Arc<RwLock<Option<Debouncer<RecommendedWatcher, FileIdMap>>>>,
	on_file_change: Arc<Notify>,
	pending_scan: Arc<Notify>,
	status: Arc<RwLock<Status>>,
	parameters: Arc<RwLock<Option<Parameters>>>,
}

impl Scanner {
	pub async fn new(
		index_manager: index::Manager,
		config_manager: config::Manager,
	) -> Result<Self, Error> {
		let scanner = Self {
			index_manager,
			config_manager: config_manager.clone(),
			file_watcher: Arc::default(),
			on_file_change: Arc::default(),
			pending_scan: Arc::new(Notify::new()),
			status: Arc::new(RwLock::new(Status::default())),
			parameters: Arc::default(),
		};

		let abort_scan = Arc::new(Notify::new());

		tokio::spawn({
			let scanner = scanner.clone();
			let abort_scan = abort_scan.clone();
			async move {
				loop {
					scanner.wait_for_change().await;
					abort_scan.notify_waiters();
					scanner.status.write().await.state = State::Pending;
					while tokio::time::timeout(Duration::from_secs(2), scanner.wait_for_change())
						.await
						.is_ok()
					{}
					scanner.pending_scan.notify_waiters();
				}
			}
		});

		tokio::spawn({
			let scanner = scanner.clone();
			async move {
				loop {
					scanner.pending_scan.notified().await;
					tokio::select! {
						result = scanner.run_scan() => {
							if let Err(e) = result {
								error!("Error while updating index: {e}");
							}
						}
						_ = abort_scan.notified() => {
							info!("Interrupted index update");
						}
					};
				}
			}
		});

		Ok(scanner)
	}

	async fn setup_file_watcher(
		config_manager: &config::Manager,
		on_file_changed: Arc<Notify>,
	) -> Result<Debouncer<RecommendedWatcher, FileIdMap>, Error> {
		let mut debouncer =
			notify_debouncer_full::new_debouncer(Duration::from_millis(100), None, move |_| {
				on_file_changed.notify_waiters();
			})?;

		let mount_dirs = config_manager.get_mounts().await;
		for mount_dir in &mount_dirs {
			if let Err(e) = debouncer
				.watcher()
				.watch(&mount_dir.source, notify::RecursiveMode::Recursive)
			{
				error!("Failed to setup file watcher for `{mount_dir:#?}`: {e}");
			}
		}

		Ok(debouncer)
	}

	async fn wait_for_change(&self) {
		tokio::select! {
			_ = async {
				loop {
					self.config_manager.on_config_change().await;
					if *self.parameters.read().await == Some(self.read_parameters().await) {
						continue;
					}
					break;
				}
			} => {},
			_ = self.on_file_change.notified() => {},
		}
	}

	async fn read_parameters(&self) -> Parameters {
		let album_art_pattern = self.config_manager.get_index_album_art_pattern().await;
		let artwork_regex = Regex::new(&format!("(?i){}", &album_art_pattern)).ok();
		Parameters {
			artwork_regex,
			mount_dirs: self.config_manager.get_mounts().await,
		}
	}

	pub async fn get_status(&self) -> Status {
		self.status.read().await.clone()
	}

	pub fn queue_scan(&self) {
		self.pending_scan.notify_one();
	}

	pub fn try_trigger_scan(&self) {
		self.pending_scan.notify_waiters();
	}

	pub async fn run_scan(&self) -> Result<(), Error> {
		info!("Beginning collection scan");

		let start = Instant::now();
		{
			let mut status = self.status.write().await;
			status.last_start_time = Some(SystemTime::now());
			status.state = State::InProgress;
			status.num_songs_indexed = 0;
		}

		let was_empty = self.index_manager.is_index_empty().await;
		let mut partial_update_time = Instant::now();

		let new_parameters = self.read_parameters().await;
		*self.parameters.write().await = Some(new_parameters.clone());

		let (scan_directories_output, collection_directories_input) = channel();
		let (scan_songs_output, collection_songs_input) = channel();
		let scan = Scan::new(scan_directories_output, scan_songs_output, new_parameters);

		let mut scan_task_set = JoinSet::new();
		let mut index_task_set = JoinSet::new();
		let mut watch_task_set = JoinSet::<Result<(), Error>>::new();
		let mut secondary_task_set = JoinSet::new();

		scan_task_set.spawn_blocking(|| scan.run());

		watch_task_set.spawn({
			let scanner = self.clone();
			let config_manager = self.config_manager.clone();
			async move {
				let mut watcher = scanner.file_watcher.write().await;
				*watcher = None; // Drops previous watcher
				*watcher = Some(
					Self::setup_file_watcher(&config_manager, scanner.on_file_change.clone())
						.await?,
				);
				Ok(())
			}
		});

		let partial_index_notify = Arc::new(tokio::sync::Notify::new());
		let partial_index_mutex = Arc::new(tokio::sync::Mutex::new(index::Builder::default()));
		secondary_task_set.spawn({
			let index_manager = self.index_manager.clone();
			let partial_index_notify = partial_index_notify.clone();
			let partial_index_mutex = partial_index_mutex.clone();
			async move {
				loop {
					partial_index_notify.notified().await;
					let mut partial_index = partial_index_mutex.clone().lock_owned().await;
					let partial_index =
						std::mem::replace(&mut *partial_index, index::Builder::new());
					let partial_index = partial_index.build();
					let num_songs = partial_index.collection.num_songs();
					index_manager.clone().replace_index(partial_index).await;
					info!("Promoted partial collection index ({num_songs} songs)");
				}
			}
		});

		let (status_sender, mut status_receiver) = unbounded_channel();
		secondary_task_set.spawn({
			let manager = self.clone();
			async move {
				while let Some(n) = status_receiver.recv().await {
					manager.status.write().await.num_songs_indexed = n;
				}
			}
		});

		index_task_set.spawn_blocking(move || {
			let mut index_builder = index::Builder::default();
			let mut num_songs_scanned = 0;

			loop {
				let exhausted_songs = match collection_songs_input.try_recv() {
					Ok(song) => {
						index_builder.add_song(song);
						num_songs_scanned += 1;
						status_sender.send(num_songs_scanned).ok();
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

				if was_empty && partial_update_time.elapsed().as_secs() > 5 {
					if let Ok(mut m) = partial_index_mutex.clone().try_lock_owned() {
						*m = index_builder.clone();
						partial_index_notify.notify_one();
						partial_update_time = Instant::now()
					}
				}
			}

			index_builder.build()
		});

		scan_task_set.join_next().await.unwrap()??;
		watch_task_set.join_next().await.unwrap()??;
		let index = index_task_set.join_next().await.unwrap()?;
		secondary_task_set.abort_all();

		self.index_manager.persist_index(&index).await?;
		self.index_manager.replace_index(index).await;

		{
			let mut status = self.status.write().await;
			status.state = State::UpToDate;
			status.last_end_time = Some(SystemTime::now());
		}

		info!(
			"Collection scan took {} seconds",
			start.elapsed().as_millis() as f32 / 1000.0
		);

		Ok(())
	}
}

struct Scan {
	directories_output: Sender<Directory>,
	songs_output: Sender<Song>,
	parameters: Parameters,
}

impl Scan {
	pub fn new(
		directories_output: Sender<Directory>,
		songs_output: Sender<Song>,
		parameters: Parameters,
	) -> Self {
		Self {
			directories_output,
			songs_output,
			parameters,
		}
	}

	pub fn run(self) -> Result<(), Error> {
		let key = "POLARIS_NUM_TRAVERSER_THREADS";
		let num_threads = std::env::var_os(key)
			.map(|v| v.to_string_lossy().to_string())
			.and_then(|v| usize::from_str(&v).ok())
			.unwrap_or_else(|| min(num_cpus::get(), 8));
		info!("Browsing collection using {} threads", num_threads);

		let directories_output = self.directories_output.clone();
		let songs_output = self.songs_output.clone();
		let artwork_regex = self.parameters.artwork_regex.clone();

		let thread_pool = ThreadPoolBuilder::new().num_threads(num_threads).build()?;
		thread_pool.scope({
			|scope| {
				for mount in self.parameters.mount_dirs {
					scope.spawn(|scope| {
						process_directory(
							scope,
							mount.source,
							mount.name,
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
	directories_output: Sender<Directory>,
	songs_output: Sender<Song>,
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
		let entry = match entry {
			Ok(e) => e,
			Err(e) => {
				error!(
					"File read error within `{}`: {}",
					real_path.as_ref().display(),
					e
				);
				continue;
			}
		};

		let is_dir = match entry.file_type().map(|f| f.is_dir()) {
			Ok(d) => d,
			Err(e) => {
				error!(
					"Could not determine file type for `{}`: {}",
					entry.path().to_string_lossy(),
					e
				);
				continue;
			}
		};
		let name = entry.file_name();
		let entry_real_path = real_path.as_ref().join(&name);
		let entry_virtual_path = virtual_path.as_ref().join(&name);

		if is_dir {
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
				real_path: entry_real_path.clone(),
				virtual_path: entry_virtual_path.clone(),
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

	use crate::app::test::{self};
	use crate::test_name;

	use super::*;

	#[tokio::test]
	async fn scan_finds_songs_and_directories() {
		let (directories_sender, directories_receiver) = channel();
		let (songs_sender, songs_receiver) = channel();
		let parameters = Parameters {
			artwork_regex: None,
			mount_dirs: vec![config::MountDir {
				source: ["test-data", "small-collection"].iter().collect(),
				name: "root".to_owned(),
			}],
		};

		let scan = Scan::new(directories_sender, songs_sender, parameters);
		scan.run().unwrap();

		let directories = directories_receiver.iter().collect::<Vec<_>>();
		assert_eq!(directories.len(), 6);

		let songs = songs_receiver.iter().collect::<Vec<_>>();
		assert_eq!(songs.len(), 13);
	}

	#[tokio::test]
	async fn scan_finds_embedded_artwork() {
		let (directories_sender, _) = channel();
		let (songs_sender, songs_receiver) = channel();
		let parameters = Parameters {
			artwork_regex: None,
			mount_dirs: vec![config::MountDir {
				source: ["test-data", "small-collection"].iter().collect(),
				name: "root".to_owned(),
			}],
		};

		let scan = Scan::new(directories_sender, songs_sender, parameters);
		scan.run().unwrap();

		let songs = songs_receiver.iter().collect::<Vec<_>>();

		songs
			.iter()
			.any(|s| s.artwork.as_ref() == Some(&s.virtual_path));
	}

	#[tokio::test]
	async fn album_art_pattern_is_case_insensitive() {
		let artwork_path = PathBuf::from_iter(["root", "Khemmis", "Hunted", "Folder.jpg"]);
		let patterns = vec!["folder", "FOLDER"];
		for pattern in patterns.into_iter() {
			let (directories_sender, _) = channel();
			let (songs_sender, songs_receiver) = channel();
			let parameters = Parameters {
				artwork_regex: Some(Regex::new(pattern).unwrap()),
				mount_dirs: vec![config::MountDir {
					source: ["test-data", "small-collection"].iter().collect(),
					name: "root".to_owned(),
				}],
			};

			let scan = Scan::new(directories_sender, songs_sender, parameters);
			scan.run().unwrap();

			let songs = songs_receiver.iter().collect::<Vec<_>>();

			songs
				.iter()
				.any(|s| s.artwork.as_ref() == Some(&artwork_path));
		}
	}

	#[tokio::test]
	async fn scanner_reacts_to_config_changes() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;

		assert!(ctx.index_manager.is_index_empty().await);

		ctx.config_manager
			.set_mounts(vec![config::storage::MountDir {
				source: ["test-data", "small-collection"].iter().collect(),
				name: "root".to_owned(),
			}])
			.await
			.unwrap();

		tokio::time::timeout(Duration::from_secs(10), async {
			loop {
				tokio::time::sleep(Duration::from_millis(100)).await;
				if !ctx.index_manager.is_index_empty().await {
					break;
				}
			}
		})
		.await
		.expect("Index did not populate");
	}
}
