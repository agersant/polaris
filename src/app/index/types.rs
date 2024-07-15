use std::path::Path;

use crate::app::vfs::VFS;

#[derive(Debug, PartialEq, Eq)]
pub struct MultiString(pub Vec<String>);

#[derive(Debug, PartialEq, Eq)]
pub enum CollectionFile {
	Directory(Directory),
	Song(Song),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Song {
	pub id: i64,
	pub path: String,
	pub parent: String,
	pub track_number: Option<i64>,
	pub disc_number: Option<i64>,
	pub title: Option<String>,
	pub artists: MultiString,
	pub album_artists: MultiString,
	pub year: Option<i64>,
	pub album: Option<String>,
	pub artwork: Option<String>,
	pub duration: Option<i64>,
	pub lyricists: MultiString,
	pub composers: MultiString,
	pub genres: MultiString,
	pub labels: MultiString,
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

#[derive(Debug, PartialEq, Eq)]
pub struct Directory {
	pub id: i64,
	pub path: String,
	pub parent: Option<String>,
	pub artists: MultiString,
	pub year: Option<i64>,
	pub album: Option<String>,
	pub artwork: Option<String>,
	pub date_added: i64,
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
