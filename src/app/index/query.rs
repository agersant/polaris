use diesel::dsl::sql;
use diesel::sql_types;
use diesel::{alias, prelude::*};
use std::path::{Path, PathBuf};

use super::*;
use crate::db::{self, artists, directories, song_album_artists, song_artists, songs};
use crate::service::dto;

#[derive(thiserror::Error, Debug)]
pub enum QueryError {
	#[error(transparent)]
	Database(#[from] diesel::result::Error),
	#[error(transparent)]
	DatabaseConnection(#[from] db::Error),
	#[error("Song was not found: `{0}`")]
	SongNotFound(PathBuf),
	#[error(transparent)]
	Vfs(#[from] vfs::Error),
}

sql_function!(
	#[aggregate]
	fn random() -> Integer;
);

impl Index {
	pub fn browse<P>(&self, virtual_path: P) -> Result<Vec<dto::CollectionFile>, QueryError>
	where
		P: AsRef<Path>,
	{
		let mut output = Vec::new();
		let vfs = self.vfs_manager.get_vfs()?;
		let mut connection = self.db.connect()?;

		if virtual_path.as_ref().components().count() == 0 {
			// Browse top-level
			let real_directories: Vec<Directory> = directories::table
				.filter(directories::parent.is_null())
				.load(&mut connection)?;

			let virtual_directories = real_directories
				.into_iter()
				.filter_map(|d| d.virtualize(&vfs))
				.map(|d| d.fetch_artists(&mut connection))
				.map(|d| d.map(dto::CollectionFile::Directory));

			for d in virtual_directories {
				output.push(d?);
			}
		} else {
			// Browse sub-directory
			let real_path = vfs.virtual_to_real(virtual_path)?;
			let real_path_string = real_path.as_path().to_string_lossy().into_owned();

			let real_directories: Vec<Directory> = directories::table
				.filter(directories::parent.eq(&real_path_string))
				.order(sql::<sql_types::Bool>("path COLLATE NOCASE ASC"))
				.load(&mut connection)?;

			let real_songs: Vec<Song> = songs::table
				.filter(songs::parent.eq(&real_path_string))
				.order(sql::<sql_types::Bool>("path COLLATE NOCASE ASC"))
				.load(&mut connection)?;

			// Preallocate capacity
			output.reserve(real_directories.len() + real_songs.len());

			let virtual_directories = real_directories
				.into_iter()
				.filter_map(|d| d.virtualize(&vfs))
				.map(|s| s.fetch_artists(&mut connection))
				.map(|s| s.map(dto::CollectionFile::Directory));
			for d in virtual_directories {
				output.push(d?);
			}

			let virtual_songs = real_songs
				.into_iter()
				.filter_map(|s| s.virtualize(&vfs))
				.map(|s| s.fetch_artists(&mut connection))
				.map(|s| s.map(dto::CollectionFile::Song));
			for d in virtual_songs {
				output.push(d?);
			}
		}

		Ok(output)
	}

	pub fn flatten<P>(&self, virtual_path: P) -> Result<Vec<dto::Song>, QueryError>
	where
		P: AsRef<Path>,
	{
		use self::songs::dsl::*;
		let vfs = self.vfs_manager.get_vfs()?;
		let mut connection = self.db.connect()?;

		let real_songs: Vec<Song> = if virtual_path.as_ref().parent().is_some() {
			let real_path = vfs.virtual_to_real(virtual_path)?;
			let song_path_filter = {
				let mut path_buf = real_path;
				path_buf.push("%");
				path_buf.as_path().to_string_lossy().into_owned()
			};
			songs
				.filter(path.like(&song_path_filter))
				.order(path)
				.load(&mut connection)?
		} else {
			songs.order(path).load(&mut connection)?
		};

		let virtual_songs = real_songs
			.into_iter()
			.filter_map(|s| s.virtualize(&vfs))
			.map(|s| s.fetch_artists(&mut connection));

		Ok(virtual_songs.collect::<Result<_, _>>()?)
	}

	pub fn get_random_albums(&self, count: i64) -> Result<Vec<dto::Directory>, QueryError> {
		use self::directories::dsl::*;
		let vfs = self.vfs_manager.get_vfs()?;
		let mut connection = self.db.connect()?;
		let real_directories: Vec<Directory> = directories
			.filter(album.is_not_null())
			.limit(count)
			.order(random())
			.load(&mut connection)?;

		let virtual_directories = real_directories
			.into_iter()
			.filter_map(|d| d.virtualize(&vfs))
			.map(|d| d.fetch_artists(&mut connection));

		Ok(virtual_directories.collect::<Result<_, _>>()?)
	}

	pub fn get_recent_albums(&self, count: i64) -> Result<Vec<dto::Directory>, QueryError> {
		use self::directories::dsl::*;
		let vfs = self.vfs_manager.get_vfs()?;
		let mut connection = self.db.connect()?;
		let real_directories: Vec<Directory> = directories
			.filter(album.is_not_null())
			.order(date_added.desc())
			.limit(count)
			.load(&mut connection)?;

		let virtual_directories = real_directories
			.into_iter()
			.filter_map(|d| d.virtualize(&vfs))
			.map(|d| d.fetch_artists(&mut connection));

		Ok(virtual_directories.collect::<Result<_, _>>()?)
	}

	pub fn search(&self, query: &str) -> Result<Vec<dto::CollectionFile>, QueryError> {
		let vfs = self.vfs_manager.get_vfs()?;
		let mut connection = self.db.connect()?;
		let like_test = format!("%{}%", query);
		let mut output = Vec::new();

		// Find dirs with matching path and parent not matching
		let real_directories: Vec<Directory> = {
			use self::directories::dsl::*;
			directories
				.filter(path.like(&like_test))
				.filter(parent.not_like(&like_test))
				.load(&mut connection)?
		};

		// Find songs with matching title/album/artist and non-matching parent
		let real_songs: Vec<Song> = {
			use self::songs::dsl::*;

			let album_artists = alias!(artists as album_artists);
			songs
				.select(songs::all_columns())
				.left_join(song_artists::table)
				.left_join(artists::table.on(song_artists::artist.eq(artists::id)))
				.left_join(song_album_artists::table)
				.left_join(
					album_artists
						.on(song_album_artists::artist.eq(album_artists.field(artists::id))),
				)
				.filter(
					path.like(&like_test)
						.or(title.like(&like_test))
						.or(album.like(&like_test))
						.or(artists::name.like(&like_test))
						.or(album_artists.field(artists::name).like(&like_test)),
				)
				.filter(parent.not_like(&like_test))
				.load(&mut connection)?
		};

		// Preallocate capacity
		output.reserve(real_directories.len() + real_songs.len());

		let virtual_directories = real_directories
			.into_iter()
			.filter_map(|d| d.virtualize(&vfs))
			.map(|d| d.fetch_artists(&mut connection))
			.map(|d| d.map(dto::CollectionFile::Directory));
		for d in virtual_directories {
			output.push(d?);
		}

		let virtual_songs = real_songs
			.into_iter()
			.filter_map(|d| d.virtualize(&vfs))
			.map(|s| s.fetch_artists(&mut connection))
			.map(|s| s.map(dto::CollectionFile::Song));
		for s in virtual_songs {
			output.push(s?);
		}

		Ok(output)
	}

	pub fn get_song(&self, virtual_path: &Path) -> Result<dto::Song, QueryError> {
		let vfs = self.vfs_manager.get_vfs()?;
		let mut connection = self.db.connect()?;

		let real_path = vfs.virtual_to_real(virtual_path)?;
		let real_path_string = real_path.as_path().to_string_lossy();

		use self::songs::dsl::*;
		let real_song: Song = songs
			.filter(path.eq(real_path_string))
			.get_result(&mut connection)?;

		let virtual_song = match real_song.virtualize(&vfs) {
			Some(s) => s,
			None => return Err(QueryError::SongNotFound(real_path)),
		};

		Ok(virtual_song.fetch_artists(&mut connection)?)
	}
}
