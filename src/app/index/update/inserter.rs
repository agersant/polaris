use crossbeam_channel::Receiver;
use diesel::prelude::*;
use log::error;

use crate::app::index::metadata::SongTags;
use crate::app::index::QueryError;
use crate::db::{
	artists, directories, directory_artists, song_album_artists, song_artists, songs, DB,
};

const INDEX_BUILDING_INSERT_BUFFER_SIZE: usize = 1000; // Insertions in each transaction

#[derive(Debug, Insertable, AsChangeset)]
#[diesel(table_name = artists)]
pub struct Artist {
	pub name: String,
}

pub struct InsertSong {
	pub path: String,
	pub parent: String,
	pub artwork: Option<String>,
	pub tags: SongTags,
}

#[derive(Debug, Insertable, AsChangeset)]
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

pub struct InsertDirectory {
	pub path: String,
	pub parent: Option<String>,
	pub artists: Vec<String>,
	pub year: Option<i32>,
	pub album: Option<String>,
	pub artwork: Option<String>,
	pub date_added: i32,
}

#[derive(Debug, Insertable, AsChangeset)]
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
		let res = self
			.db
			.connect()
			.map_err(QueryError::from)
			.and_then(|mut connection| {
				for d in self.new_directories.drain(..) {
					let dir = Directory {
						path: d.path,
						parent: d.parent,
						artwork: d.artwork,
						album: d.album,
						year: d.year,
						date_added: d.date_added,
					};
					let dir_id: i32 = diesel::insert_into(directories::table)
						.values(&dir)
						.on_conflict(directories::path)
						.do_update()
						.set(&dir)
						.returning(directories::id)
						.get_result(&mut connection)?;

					for a in d.artists {
						let artist = Artist { name: a };
						let artist_id: i32 = diesel::insert_into(artists::table)
							.values(&artist)
							.on_conflict(artists::name)
							.do_update()
							.set(&artist)
							.returning(artists::id)
							.get_result(&mut connection)?;

						let dir_artist = DirectoryArtist {
							directory: dir_id,
							artist: artist_id,
						};
						diesel::insert_into(directory_artists::table)
							.values(dir_artist)
							.execute(&mut *connection)?;
					}
				}

				Ok(())
			});

		if let Err(e) = res {
			error!("Could not insert new directories in database: {e}");
		}
	}

	fn flush_songs(&mut self) {
		let res = self
			.db
			.connect()
			.map_err(QueryError::from)
			.and_then(|mut connection| {
				for s in self.new_songs.drain(..) {
					let song = Song {
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
					};
					let song_id: i32 = diesel::insert_into(songs::table)
						.values(&song)
						.on_conflict(songs::path)
						.do_update()
						.set(&song)
						.returning(songs::id)
						.get_result(&mut connection)?;

					for a in s.tags.artists {
						let artist = Artist { name: a };
						let artist_id: i32 = diesel::insert_into(artists::table)
							.values(&artist)
							.on_conflict(artists::name)
							.do_update()
							.set(&artist)
							.returning(artists::id)
							.get_result(&mut connection)?;

						let song_artist = SongArtist {
							song: song_id,
							artist: artist_id,
						};
						diesel::insert_into(song_artists::table)
							.values(song_artist)
							.execute(&mut connection)?;
					}

					for a in s.tags.album_artists {
						let artist = Artist { name: a };
						let artist_id: i32 = diesel::insert_into(artists::table)
							.values(&artist)
							.on_conflict(artists::name)
							.do_update()
							.set(&artist)
							.returning(artists::id)
							.get_result(&mut connection)?;

						let song_album_artist = SongAlbumArtist {
							song: song_id,
							artist: artist_id,
						};
						diesel::insert_into(song_album_artists::table)
							.values(song_album_artist)
							.execute(&mut connection)?;
					}
				}

				Ok(())
			});

		if let Err(e) = res {
			error!("Could not insert new songs in database: {e}");
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
