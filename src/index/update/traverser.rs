use crossbeam_channel::Sender;
use log::error;
use std::fs;
use std::path::{Path, PathBuf};

use crate::index::metadata::{self, SongTags};

pub struct Song {
	pub path: PathBuf,
	pub metadata: SongTags,
}

pub struct Directory {
	pub parent: Option<PathBuf>,
	pub path: PathBuf,
	pub songs: Vec<Song>,
	pub other_files: Vec<PathBuf>,
	pub created: i32,
}

pub struct Traverser {
	output: Sender<Directory>,
}

impl Traverser {
	pub fn new(output: Sender<Directory>) -> Self {
		Self { output }
	}

	pub fn traverse(&self, parent: Option<&Path>, dir: &Path) {
		#[cfg(feature = "profile-index")]
		let _guard = flame::start_guard(format!(
			"traverse: {}",
			dir.file_name()
				.map(|s| { s.to_string_lossy().into_owned() })
				.unwrap_or("Unknown".to_owned())
		));

		let read_dir = match fs::read_dir(dir) {
			Ok(read_dir) => read_dir,
			Err(e) => {
				error!("Directory read error for `{}`: {}", dir.display(), e);
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
					error!("File read error within `{}`: {}", dir.display(), e);
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

		let created = Self::get_date_created(dir).unwrap_or_default();

		if let Err(e) = self.output.send(Directory {
			path: dir.to_owned(),
			parent: parent.map(|p| p.to_owned()),
			songs,
			other_files,
			created,
		}) {
			error!("Error emitting directory `{}`: {}", dir.display(), e);
		}

		for sub_directory in sub_directories {
			self.traverse(Some(dir), &sub_directory);
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
