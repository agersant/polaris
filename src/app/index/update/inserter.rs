use std::borrow::Cow;

use log::error;
use sqlx::{
	encode::IsNull,
	sqlite::{SqliteArgumentValue, SqliteTypeInfo},
	QueryBuilder, Sqlite,
};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::{app::index::MultiString, db::DB};

const INDEX_BUILDING_INSERT_BUFFER_SIZE: usize = 1000; // Insertions in each transaction

pub struct Song {
	pub path: String,
	pub parent: String,
	pub track_number: Option<i32>,
	pub disc_number: Option<i32>,
	pub title: Option<String>,
	pub artists: MultiString,
	pub album_artists: MultiString,
	pub year: Option<i32>,
	pub album: Option<String>,
	pub artwork: Option<String>,
	pub duration: Option<i32>,
	pub lyricists: MultiString,
	pub composers: MultiString,
	pub genres: MultiString,
	pub labels: MultiString,
}

pub struct Directory {
	pub path: String,
	pub parent: Option<String>,
	pub artists: MultiString,
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

static MULTI_STRING_SEPARATOR: &str = "\u{000C}";

impl<'q> sqlx::Encode<'q, Sqlite> for MultiString {
	fn encode_by_ref(&self, args: &mut Vec<SqliteArgumentValue<'q>>) -> IsNull {
		if self.0.is_empty() {
			IsNull::Yes
		} else {
			let joined = self.0.join(MULTI_STRING_SEPARATOR);
			args.push(SqliteArgumentValue::Text(Cow::Owned(joined)));
			IsNull::No
		}
	}
}

impl From<Option<String>> for MultiString {
	fn from(value: Option<String>) -> Self {
		match value {
			None => MultiString(Vec::new()),
			Some(s) => MultiString(
				s.split(MULTI_STRING_SEPARATOR)
					.map(|s| s.to_string())
					.collect(),
			),
		}
	}
}

impl sqlx::Type<Sqlite> for MultiString {
	fn type_info() -> SqliteTypeInfo {
		<&str as sqlx::Type<Sqlite>>::type_info()
	}
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
			"INSERT INTO directories(path, parent, artists, year, album, artwork, date_added) ",
		)
		.push_values(&self.new_directories, |mut b, directory| {
			b.push_bind(&directory.path)
				.push_bind(&directory.parent)
				.push_bind(&directory.artists)
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

		let result = QueryBuilder::<Sqlite>::new("INSERT INTO songs(path, parent, track_number, disc_number, title, artists, album_artists, year, album, artwork, duration, lyricists, composers, genres, labels) ")
		.push_values(&self.new_songs, |mut b, song| {
			b.push_bind(&song.path)
				.push_bind(&song.parent)
				.push_bind(song.track_number)
				.push_bind(song.disc_number)
				.push_bind(&song.title)
				.push_bind(&song.artists)
				.push_bind(&song.album_artists)
				.push_bind(song.year)
				.push_bind(&song.album)
				.push_bind(&song.artwork)
				.push_bind(song.duration)
				.push_bind(&song.lyricists)
				.push_bind(&song.composers)
				.push_bind(&song.genres)
				.push_bind(&song.labels);
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
