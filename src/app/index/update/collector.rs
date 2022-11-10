use crossbeam_channel::{Receiver, Sender};
use log::error;
use regex::Regex;

use super::*;

pub struct Collector {
	receiver: Receiver<traverser::Directory>,
	sender: Sender<inserter::Item>,
	album_art_pattern: Option<Regex>,
}

impl Collector {
	pub fn new(
		receiver: Receiver<traverser::Directory>,
		sender: Sender<inserter::Item>,
		album_art_pattern: Option<Regex>,
	) -> Self {
		Self {
			receiver,
			sender,
			album_art_pattern,
		}
	}

	pub fn collect(&self) {
		while let Ok(directory) = self.receiver.recv() {
			self.collect_directory(directory);
		}
	}

	fn collect_directory(&self, directory: traverser::Directory) {
		let mut directory_album = None;
		let mut directory_year = None;
		let mut directory_artists = Vec::new();
		let mut inconsistent_directory_album = false;
		let mut inconsistent_directory_year = false;
		let mut inconsistent_directory_artists = false;

		let directory_artwork = self.get_artwork(&directory);
		let directory_path_string = directory.path.to_string_lossy().to_string();
		let directory_parent_string = directory.parent.map(|p| p.to_string_lossy().to_string());

		for song in directory.songs {
			let tags = song.metadata;
			let path_string = song.path.to_string_lossy().to_string();

			if tags.year.is_some() {
				inconsistent_directory_year |=
					directory_year.is_some() && directory_year != tags.year;
				directory_year = tags.year;
			}

			if tags.album.is_some() {
				inconsistent_directory_album |=
					directory_album.is_some() && directory_album != tags.album;
				directory_album = tags.album.as_ref().cloned();
			}

			if tags.album_artists.is_empty() {
				inconsistent_directory_artists |=
					directory_artists.is_empty() && directory_artists != tags.album_artists;
				directory_artists = tags.album_artists.clone();
			} else if tags.artists.is_empty() {
				inconsistent_directory_artists |=
					directory_artists.is_empty() && directory_artists != tags.artists;
				directory_artists = tags.artists.clone();
			}

			let artwork_path = if tags.has_artwork {
				Some(path_string.clone())
			} else {
				directory_artwork.as_ref().cloned()
			};

			if let Err(e) = self.sender.send(inserter::Item::Song(inserter::InsertSong {
				path: path_string,
				parent: directory_path_string.clone(),
				artwork: artwork_path,
				tags,
			})) {
				error!("Error while sending song from collector: {}", e);
			}
		}

		if inconsistent_directory_year {
			directory_year = None;
		}
		if inconsistent_directory_album {
			directory_album = None;
		}
		if inconsistent_directory_artists {
			directory_artists = Vec::new();
		}

		if let Err(e) = self
			.sender
			.send(inserter::Item::Directory(inserter::InsertDirectory {
				path: directory_path_string,
				parent: directory_parent_string,
				artwork: directory_artwork,
				album: directory_album,
				artists: directory_artists,
				year: directory_year,
				date_added: directory.created,
			})) {
			error!("Error while sending directory from collector: {}", e);
		}
	}

	fn get_artwork(&self, directory: &traverser::Directory) -> Option<String> {
		let regex_artwork = directory.other_files.iter().find_map(|path| {
			let matches = path
				.file_name()
				.and_then(|name| name.to_str())
				.map(|name| match &self.album_art_pattern {
					Some(pattern) => pattern.is_match(name),
					None => false,
				})
				.unwrap_or(false);
			if matches {
				Some(path.to_string_lossy().to_string())
			} else {
				None
			}
		});

		let embedded_artwork = directory.songs.iter().find_map(|song| {
			if song.metadata.has_artwork {
				Some(song.path.to_string_lossy().to_string())
			} else {
				None
			}
		});

		regex_artwork.or(embedded_artwork)
	}
}
