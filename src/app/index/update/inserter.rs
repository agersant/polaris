use log::error;
use sqlx::{QueryBuilder, Sqlite};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::db::DB;

const INDEX_BUILDING_INSERT_BUFFER_SIZE: usize = 1000; // Insertions in each transaction

pub struct Song {
	pub path: String,
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

pub struct Directory {
	pub path: String,
	pub parent: Option<String>,
	pub artist: Option<String>,
	pub year: Option<i32>,
	pub album: Option<String>,
	pub artwork: Option<String>,
	pub date_added: i32,
}

pub enum Item {
	Directory(Directory),
	Song(Song),
}

pub struct Inserter {
	receiver: UnboundedReceiver<Item>,
	new_directories: Vec<Directory>,
	new_songs: Vec<Song>,
	db: DB,
}

impl Inserter {
	pub fn new(db: DB, receiver: UnboundedReceiver<Item>) -> Self {
		let new_directories = Vec::with_capacity(INDEX_BUILDING_INSERT_BUFFER_SIZE);
		let new_songs = Vec::with_capacity(INDEX_BUILDING_INSERT_BUFFER_SIZE);
		Self {
			receiver,
			new_directories,
			new_songs,
			db,
		}
	}

	pub async fn insert(&mut self) {
		while let Some(item) = self.receiver.recv().await {
			self.insert_item(item).await;
		}
		self.flush_directories().await;
		self.flush_songs().await;
	}

	async fn insert_item(&mut self, insert: Item) {
		match insert {
			Item::Directory(d) => {
				self.new_directories.push(d);
				if self.new_directories.len() >= INDEX_BUILDING_INSERT_BUFFER_SIZE {
					self.flush_directories().await;
				}
			}
			Item::Song(s) => {
				self.new_songs.push(s);
				if self.new_songs.len() >= INDEX_BUILDING_INSERT_BUFFER_SIZE {
					self.flush_songs().await;
				}
			}
		};
	}

	async fn flush_directories(&mut self) {
		let Ok(mut connection) = self.db.connect().await else {
			error!("Could not acquire connection to insert new directories in database");
			return;
		};

		let result = QueryBuilder::<Sqlite>::new(
			"INSERT INTO directories(path, parent, artist, year, album, artwork, date_added) ",
		)
		.push_values(&self.new_directories, |mut b, directory| {
			b.push_bind(&directory.path)
				.push_bind(&directory.parent)
				.push_bind(&directory.artist)
				.push_bind(directory.year)
				.push_bind(&directory.album)
				.push_bind(&directory.artwork)
				.push_bind(directory.date_added);
		})
		.build()
		.execute(connection.as_mut())
		.await;

		match result {
			Ok(_) => self.new_directories.clear(),
			Err(_) => error!("Could not insert new directories in database"),
		};
	}

	async fn flush_songs(&mut self) {
		let Ok(mut connection) = self.db.connect().await else {
			error!("Could not acquire connection to insert new songs in database");
			return;
		};

		let result = QueryBuilder::<Sqlite>::new("INSERT INTO songs(path, parent, track_number, disc_number, title, artist, album_artist, year, album, artwork, duration, lyricist, composer, genre, label) ")
		.push_values(&self.new_songs, |mut b, song| {
			b.push_bind(&song.path)
				.push_bind(&song.parent)
				.push_bind(song.track_number)
				.push_bind(song.disc_number)
				.push_bind(&song.title)
				.push_bind(&song.artist)
				.push_bind(&song.album_artist)
				.push_bind(song.year)
				.push_bind(&song.album)
				.push_bind(&song.artwork)
				.push_bind(song.duration)
				.push_bind(&song.lyricist)
				.push_bind(&song.composer)
				.push_bind(&song.genre)
				.push_bind(&song.label);
		})
		.build()
		.execute(connection.as_mut())
		.await;

		match result {
			Ok(_) => self.new_songs.clear(),
			Err(_) => error!("Could not insert new songs in database"),
		};
	}
}
