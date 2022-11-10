use anyhow::Error;
use crossbeam_channel::Receiver;
use diesel::prelude::*;
use log::error;

use crate::app::index::metadata::SongTags;
use crate::db::{
	artists, directories, directory_artists, song_album_artists, song_artists, songs, DB,
};

const INDEX_BUILDING_INSERT_BUFFER_SIZE: usize = 1000; // Insertions in each transaction

pub struct InsertSong {
	pub path: String,
	pub parent: String,
	pub artwork: Option<String>,
	pub tags: SongTags,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = songs)]
pub struct Song {
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

#[derive(Debug, Insertable)]
#[diesel(table_name = song_artists)]
pub struct SongArtist {
	song: i32,
	artist: i32,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = song_album_artists)]
pub struct SongAlbumArtist {
	song: i32,
	artist: i32,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = artists)]
pub struct Artist {
	pub name: String,
}

pub struct InsertDirectory {
	pub path: String,
	pub parent: Option<String>,
	pub artists: Vec<String>,
	pub year: Option<i32>,
	pub album: Option<String>,
	pub artwork: Option<String>,
	pub date_added: i32,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = directories)]
pub struct Directory {
	pub path: String,
	pub parent: Option<String>,
	pub year: Option<i32>,
	pub album: Option<String>,
	pub artwork: Option<String>,
	pub date_added: i32,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = directory_artists)]
pub struct DirectoryArtist {
	directory: i32,
	artist: i32,
}

pub enum Item {
	Directory(InsertDirectory),
	Song(InsertSong),
}

pub struct Inserter {
	receiver: Receiver<Item>,
	new_directories: Vec<InsertDirectory>,
	new_songs: Vec<InsertSong>,
	db: DB,
}

impl Inserter {
	pub fn new(db: DB, receiver: Receiver<Item>) -> Self {
		let new_directories = Vec::with_capacity(INDEX_BUILDING_INSERT_BUFFER_SIZE);
		let new_songs = Vec::with_capacity(INDEX_BUILDING_INSERT_BUFFER_SIZE);
		Self {
			receiver,
			new_directories,
			new_songs,
			db,
		}
	}

	pub fn insert(&mut self) {
		while let Ok(item) = self.receiver.recv() {
			self.insert_item(item);
		}
	}

	fn insert_item(&mut self, insert: Item) {
		match insert {
			Item::Directory(d) => {
				self.new_directories.push(d);
				if self.new_directories.len() >= INDEX_BUILDING_INSERT_BUFFER_SIZE {
					self.flush_directories();
				}
			}
			Item::Song(s) => {
				self.new_songs.push(s);
				if self.new_songs.len() >= INDEX_BUILDING_INSERT_BUFFER_SIZE {
					self.flush_songs();
				}
			}
		};
	}

	fn flush_directories(&mut self) {
		let res = self.db.connect().and_then(|mut connection| {
			//diesel::insert_into(directories::table)
			//	.values(&self.new_directories)
			//	.execute(&mut *connection) // TODO https://github.com/diesel-rs/diesel/issues/1822
			//	.map_err(Error::new)
			todo!();
		});
		if res.is_err() {
			error!("Could not insert new directories in database");
		}
		self.new_directories.clear();
	}

	fn flush_songs(&mut self) {
		let res = self.db.connect().and_then(|mut connection| {
			let songs: Vec<Song> = self
				.new_songs
				.drain(..)
				.map(|s| {
					let artists: Vec<Artist> = s
						.tags
						.artists
						.into_iter()
						.map(|name| Artist { name })
						.collect();
					let artist_ids = diesel::insert_into(artists::table)
						.values(artists)
						.returning(artists::id)
						.execute(&mut *connection);

					let album_artists: Vec<Artist> = s
						.tags
						.album_artists
						.into_iter()
						.map(|name| Artist { name })
						.collect();
					let album_artist_ids = diesel::insert_into(artists::table)
						.values(album_artists)
						.returning(artists::id)
						.get_results(&mut *connection);

					// let song_artists = artist_ids.iter().map
					// diesel::insert_into(song_artists::table)
					//     .values(records)

					Song {
						path: s.path,
						parent: s.parent,
						disc_number: s.tags.disc_number.map(|n| n as i32),
						track_number: s.tags.track_number.map(|n| n as i32),
						title: s.tags.title,
						duration: s.tags.duration.map(|n| n as i32),
						album: s.tags.album,
						year: s.tags.year,
						artwork: s.artwork,
						lyricist: s.tags.lyricist,
						composer: s.tags.composer,
						genre: s.tags.genre,
						label: s.tags.label,
					}
				})
				.collect();

			diesel::insert_into(songs::table)
				.values(&songs)
				.execute(&mut *connection) // TODO https://github.com/diesel-rs/diesel/issues/1822
				.map_err(Error::new)
		});
		if res.is_err() {
			error!("Could not insert new songs in database");
		}
	}
}

impl Drop for Inserter {
	fn drop(&mut self) {
		if !self.new_directories.is_empty() {
			self.flush_directories();
		}
		if !self.new_songs.is_empty() {
			self.flush_songs();
		}
	}
}
