use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::app::vfs::VFS;
use crate::db::songs;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum CollectionFile {
	Directory(Directory),
	Song(Song),
}

#[derive(Debug, PartialEq, Queryable, QueryableByName, Serialize, Deserialize)]
#[table_name = "songs"]
pub struct Song {
	#[serde(skip_serializing, skip_deserializing)]
	id: i32,
	pub path: String,
	#[serde(skip_serializing, skip_deserializing)]
	pub parent: String,
	pub track_number: Option<i32>,
	pub disc_number: Option<i32>,
	pub title: Option<String>,
	pub artist: Option<String>,
	pub album_artist: Option<String>,
	pub year: Option<i32>,
	pub album: Option<String>,
	pub artwork: Option<String>,
	pub duration: Option<i32>,
	pub lyricist: Option<String>,
	pub composer: Option<String>,
	pub genre: Option<String>,
	pub label: Option<String>,
}

impl Song {
	pub fn virtualize(mut self, vfs: &VFS) -> Option<Song> {
		self.path = match vfs.real_to_virtual(Path::new(&self.path)) {
			Ok(p) => p.to_string_lossy().into_owned(),
			_ => return None,
		};
		if let Some(artwork_path) = self.artwork {
			self.artwork = match vfs.real_to_virtual(Path::new(&artwork_path)) {
				Ok(p) => Some(p.to_string_lossy().into_owned()),
				_ => None,
			};
		}
		Some(self)
	}
}

#[derive(Debug, PartialEq, Queryable, Serialize, Deserialize)]
pub struct Directory {
	#[serde(skip_serializing, skip_deserializing)]
	id: i32,
	pub path: String,
	#[serde(skip_serializing, skip_deserializing)]
	pub parent: Option<String>,
	pub artist: Option<String>,
	pub year: Option<i32>,
	pub album: Option<String>,
	pub artwork: Option<String>,
	pub date_added: i32,
}

impl Directory {
	pub fn virtualize(mut self, vfs: &VFS) -> Option<Directory> {
		self.path = match vfs.real_to_virtual(Path::new(&self.path)) {
			Ok(p) => p.to_string_lossy().into_owned(),
			_ => return None,
		};
		if let Some(artwork_path) = self.artwork {
			self.artwork = match vfs.real_to_virtual(Path::new(&artwork_path)) {
				Ok(p) => Some(p.to_string_lossy().into_owned()),
				_ => None,
			};
		}
		Some(self)
	}
}
