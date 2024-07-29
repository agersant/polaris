use std::borrow::Cow;

use log::error;
use sqlx::{
	encode::IsNull,
	pool::PoolConnection,
	sqlite::{SqliteArgumentValue, SqliteTypeInfo},
	QueryBuilder, Sqlite,
};

use crate::app::collection::{self, MultiString};
use crate::db::DB;

impl<'q> sqlx::Encode<'q, Sqlite> for MultiString {
	fn encode_by_ref(&self, args: &mut Vec<SqliteArgumentValue<'q>>) -> IsNull {
		if self.0.is_empty() {
			IsNull::Yes
		} else {
			let joined = self.0.join(MultiString::SEPARATOR);
			args.push(SqliteArgumentValue::Text(Cow::Owned(joined)));
			IsNull::No
		}
	}
}

impl<'q> sqlx::Decode<'q, Sqlite> for MultiString {
	fn decode(
		value: <Sqlite as sqlx::database::HasValueRef<'q>>::ValueRef,
	) -> Result<Self, sqlx::error::BoxDynError> {
		let s: &str = sqlx::Decode::<Sqlite>::decode(value)?;
		Ok(MultiString(
			s.split(MultiString::SEPARATOR).map(str::to_owned).collect(),
		))
	}
}

impl sqlx::Type<Sqlite> for MultiString {
	fn type_info() -> SqliteTypeInfo {
		<&str as sqlx::Type<Sqlite>>::type_info()
	}
}

pub struct Inserter<T> {
	new_entries: Vec<T>,
	db: DB,
}

impl<T> Inserter<T>
where
	T: Insertable,
{
	const BUFFER_SIZE: usize = 1000;

	pub fn new(db: DB) -> Self {
		let new_entries = Vec::with_capacity(Self::BUFFER_SIZE);
		Self { new_entries, db }
	}

	pub async fn insert(&mut self, entry: T) {
		self.new_entries.push(entry);
		if self.new_entries.len() >= Self::BUFFER_SIZE {
			self.flush().await;
		}
	}

	pub async fn flush(&mut self) {
		let Ok(connection) = self.db.connect().await else {
			error!("Could not acquire connection to insert new entries in database");
			return;
		};
		match Insertable::bulk_insert(&self.new_entries, connection).await {
			Ok(_) => self.new_entries.clear(),
			Err(e) => error!("Could not insert new entries in database: {}", e),
		};
	}
}

pub trait Insertable
where
	Self: Sized,
{
	async fn bulk_insert(
		entries: &Vec<Self>,
		connection: PoolConnection<Sqlite>,
	) -> Result<(), sqlx::Error>;
}

impl Insertable for collection::Directory {
	async fn bulk_insert(
		entries: &Vec<Self>,
		mut connection: PoolConnection<Sqlite>,
	) -> Result<(), sqlx::Error> {
		QueryBuilder::<Sqlite>::new("INSERT INTO directories(path, virtual_path, virtual_parent) ")
			.push_values(entries.iter(), |mut b, directory| {
				b.push_bind(&directory.path)
					.push_bind(&directory.virtual_path)
					.push_bind(&directory.virtual_parent);
			})
			.build()
			.execute(connection.as_mut())
			.await
			.map(|_| ())
	}
}

impl Insertable for collection::Song {
	async fn bulk_insert(
		entries: &Vec<Self>,
		mut connection: PoolConnection<Sqlite>,
	) -> Result<(), sqlx::Error> {
		QueryBuilder::<Sqlite>::new("INSERT INTO songs(path, virtual_path, virtual_parent, track_number, disc_number, title, artists, album_artists, year, album, artwork, duration, lyricists, composers, genres, labels) ")
		.push_values(entries.iter(), |mut b, song| {
			b.push_bind(&song.path)
				.push_bind(&song.virtual_path)
				.push_bind(&song.virtual_parent)
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
		.await.map(|_| ())
	}
}
