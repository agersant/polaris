use std::path::Path;

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, PooledConnection};

use crate::app::vfs::VFS;
use crate::db::{artists, directory_artists, song_artists, song_album_artists, songs};
use crate::service::dto;

#[derive(Debug, PartialEq, Eq, Queryable, QueryableByName)]
#[diesel(table_name = songs)]
pub struct Song {
	pub id: i32,
	pub path: String,
	pub parent: String,
	pub track_number: Option<i32>,
	pub disc_number: Option<i32>,
	pub title: Option<String>,
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

	pub fn fetch_artists(
		self,
		connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
	) -> Result<dto::Song, diesel::result::Error> {
		let artists: Vec<String> = song_artists::table
			.filter(song_artists::song.eq(self.id))
			.inner_join(artists::table)
			.select(artists::name)
			.load(connection)?;
		let album_artists: Vec<String> = song_album_artists::table
			.filter(song_album_artists::song.eq(self.id))
			.inner_join(artists::table)
			.select(artists::name)
			.load(connection)?;
		Ok(dto::Song::new(self, artists, album_artists))
	}
}

#[derive(Debug, PartialEq, Eq, Queryable)]
pub struct Directory {
	pub id: i32,
	pub path: String,
	pub parent: Option<String>,
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

	pub fn fetch_artists(
		self,
		connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
	) -> Result<dto::Directory, diesel::result::Error> {
		let artists: Vec<String> = directory_artists::table
			.filter(directory_artists::directory.eq(self.id))
			.inner_join(artists::table)
			.select(artists::name)
			.load(connection)?;
		Ok(dto::Directory::new(self, artists))
	}
}
