use crossbeam_channel::Sender;
use log::error;
use parking_lot::Mutex;
use std::cmp::min;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::index::metadata::{self, SongTags};

#[derive(Debug)]
pub struct Song {
	pub path: PathBuf,
	pub metadata: SongTags,
}

#[derive(Debug)]
pub struct Directory {
	pub parent: Option<PathBuf>,
	pub path: PathBuf,
	pub songs: Vec<Song>,
	pub other_files: Vec<PathBuf>,
	pub created: i32,
}

pub struct Traverser {
	directory_sender: Sender<Directory>,
}

#[derive(Debug)]
struct WorkItem {
	parent: Option<PathBuf>,
	path: PathBuf,
}

impl Traverser {
	pub fn new(directory_sender: Sender<Directory>) -> Self {
		Self { directory_sender }
	}

	pub fn traverse(&self, roots: Vec<PathBuf>) -> anyhow::Result<()> {
		let initial_work: Vec<WorkItem> = roots
			.into_iter()
			.map(|d| WorkItem {
				parent: None,
				path: d,
			})
			.collect();

		let num_pending_work_items = Arc::new(AtomicUsize::new(initial_work.len()));
		let queue = Arc::new(Mutex::new(initial_work));

		let mut threads = Vec::new();
		let num_threads = min(num_cpus::get(), 4); // TODO.index
		for _ in 0..num_threads {
			let queue = queue.clone();
			let directory_sender = self.directory_sender.clone();
			let num_pending_work_items = num_pending_work_items.clone();
			threads.push(std::thread::spawn(move || {
				let worker = Worker {
					queue,
					directory_sender,
					num_pending_work_items,
				};
				worker.run();
			}));
		}

		for thread in threads {
			thread.join().ok(); // TODO.index
		}

		Ok(())
	}
}

struct Worker {
	queue: Arc<Mutex<Vec<WorkItem>>>,
	directory_sender: Sender<Directory>,
	num_pending_work_items: Arc<AtomicUsize>,
}

impl Worker {
	fn run(&self) {
		while let Some(work_item) = self.find_work_item() {
			self.process_work_item(work_item);
			self.on_item_processed();
		}
	}

	fn find_work_item(&self) -> Option<WorkItem> {
		loop {
			if self.is_all_work_done() {
				return None;
			}
			{
				let mut queue = self.queue.lock();
				if let Some(w) = queue.pop() {
					return Some(w);
				}
			};
			thread::sleep(Duration::from_millis(1));
		}
	}

	fn is_all_work_done(&self) -> bool {
		self.num_pending_work_items.load(Ordering::SeqCst) == 0
	}

	fn queue_work(&self, work_item: WorkItem) {
		self.num_pending_work_items.fetch_add(1, Ordering::SeqCst);
		let mut queue = self.queue.lock();
		queue.push(work_item);
	}

	fn on_item_processed(&self) {
		self.num_pending_work_items.fetch_sub(1, Ordering::SeqCst);
	}

	fn emit_directory(&self, directory: Directory) {
		self.directory_sender.send(directory).unwrap(); // TODO.index
	}

	pub fn process_work_item(&self, work_item: WorkItem) {
		#[cfg(feature = "profile-index")]
		let _guard = flame::start_guard(format!(
			"traverse: {}",
			dir.file_name()
				.map(|s| { s.to_string_lossy().into_owned() })
				.unwrap_or("Unknown".to_owned())
		));

		let read_dir = match fs::read_dir(&work_item.path) {
			Ok(read_dir) => read_dir,
			Err(e) => {
				error!(
					"Directory read error for `{}`: {}",
					work_item.path.display(),
					e
				);
				return;
			}
		};

		let mut sub_directories = Vec::new();
		let mut songs = Vec::new();
		let mut other_files = Vec::new();

		for entry in read_dir {
			let path = match entry {
				Ok(ref f) => f.path(),
				Err(e) => {
					error!(
						"File read error within `{}`: {}",
						work_item.path.display(),
						e
					);
					break;
				}
			};

			if path.is_dir() {
				sub_directories.push(path);
			} else {
				if let Some(metadata) = metadata::read(&path) {
					songs.push(Song { path, metadata });
				} else {
					other_files.push(path);
				}
			}
		}

		let created = Self::get_date_created(&work_item.path).unwrap_or_default();

		self.emit_directory(Directory {
			path: work_item.path.to_owned(),
			parent: work_item.parent.map(|p| p.to_owned()),
			songs,
			other_files,
			created,
		});

		for sub_directory in sub_directories.into_iter() {
			self.queue_work(WorkItem {
				parent: Some(work_item.path.clone()),
				path: sub_directory,
			});
		}
	}

	#[cfg_attr(feature = "profile-index", flame)]
	fn get_date_created(path: &Path) -> Option<i32> {
		if let Ok(t) = fs::metadata(path).and_then(|m| m.created().or(m.modified())) {
			t.duration_since(std::time::UNIX_EPOCH)
				.map(|d| d.as_secs() as i32)
				.ok()
		} else {
			None
		}
	}
}
