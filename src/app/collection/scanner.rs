use log::{error, info};
use rayon::{Scope, ThreadPoolBuilder};
use regex::Regex;
use std::cmp::min;
use std::fs;
use std::path::Path;
use std::str::FromStr;
use tokio::sync::mpsc::UnboundedSender;

use crate::app::vfs;
use crate::app::{
	collection::{self, MultiString},
	formats,
};

pub struct Scanner {
	directories_output: UnboundedSender<collection::Directory>,
	songs_output: UnboundedSender<collection::Song>,
	vfs_manager: vfs::Manager,
	artwork_regex: Option<Regex>,
}

impl Scanner {
	pub fn new(
		directories_output: UnboundedSender<collection::Directory>,
		songs_output: UnboundedSender<collection::Song>,
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

	pub async fn scan(self) -> Result<(), collection::Error> {
		let vfs = self.vfs_manager.get_vfs().await?;
		let roots = vfs.mounts().clone();

		let key = "POLARIS_NUM_TRAVERSER_THREADS";
		let num_threads = std::env::var_os(key)
			.map(|v| v.to_string_lossy().to_string())
			.and_then(|v| usize::from_str(&v).ok())
			.unwrap_or_else(|| min(num_cpus::get(), 4));
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
	directories_output: UnboundedSender<collection::Directory>,
	songs_output: UnboundedSender<collection::Song>,
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
		let entry_real_path_string = entry_real_path.to_string_lossy().to_string();

		let entry_virtual_path = virtual_path.as_ref().join(&name);
		let entry_virtual_path_string = entry_virtual_path.to_string_lossy().to_string();

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
			songs.push(collection::Song {
				id: 0,
				path: entry_real_path_string.clone(),
				virtual_path: entry_virtual_path.to_string_lossy().to_string(),
				virtual_parent: entry_virtual_path
					.parent()
					.unwrap()
					.to_string_lossy()
					.to_string(),
				track_number: metadata.track_number.map(|n| n as i64),
				disc_number: metadata.disc_number.map(|n| n as i64),
				title: metadata.title,
				artists: MultiString(metadata.artists),
				album_artists: MultiString(metadata.album_artists),
				year: metadata.year.map(|n| n as i64),
				album: metadata.album,
				artwork: metadata
					.has_artwork
					.then(|| entry_virtual_path_string.clone()),
				duration: metadata.duration.map(|n| n as i64),
				lyricists: MultiString(metadata.lyricists),
				composers: MultiString(metadata.composers),
				genres: MultiString(metadata.genres),
				labels: MultiString(metadata.labels),
				date_added: get_date_created(&entry_real_path).unwrap_or_default(),
			});
		} else if artwork_file.is_none()
			&& artwork_regex
				.as_ref()
				.is_some_and(|r| r.is_match(name.to_str().unwrap_or_default()))
		{
			artwork_file = Some(entry_virtual_path_string);
		}
	}

	for mut song in songs {
		song.artwork = song.artwork.or_else(|| artwork_file.clone());
		songs_output.send(song).ok();
	}

	directories_output
		.send(collection::Directory {
			id: 0,
			path: real_path.as_ref().to_string_lossy().to_string(),
			virtual_path: virtual_path.as_ref().to_string_lossy().to_string(),
			virtual_parent: virtual_path
				.as_ref()
				.parent()
				.map(|p| p.to_string_lossy().to_string())
				.filter(|p| !p.is_empty()),
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
