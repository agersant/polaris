use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types;
use std::path::{Path, PathBuf};

use super::*;
use crate::db::{self, directories, songs};

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
	pub fn browse<P>(&self, virtual_path: P) -> Result<Vec<CollectionFile>, QueryError>
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
				.filter_map(|d| d.virtualize(&vfs));
			output.extend(virtual_directories.map(CollectionFile::Directory));
		} else {
			// Browse sub-directory
			let real_path = vfs.virtual_to_real(virtual_path)?;
			let real_path_string = real_path.as_path().to_string_lossy().into_owned();

			let real_directories: Vec<Directory> = directories::table
				.filter(directories::parent.eq(&real_path_string))
				.order(sql::<sql_types::Bool>("path COLLATE NOCASE ASC"))
				.load(&mut connection)?;
			let virtual_directories = real_directories
				.into_iter()
				.filter_map(|d| d.virtualize(&vfs));
			output.extend(virtual_directories.map(CollectionFile::Directory));

			let real_songs: Vec<Song> = songs::table
				.filter(songs::parent.eq(&real_path_string))
				.order(sql::<sql_types::Bool>("path COLLATE NOCASE ASC"))
				.load(&mut connection)?;
			let virtual_songs = real_songs.into_iter().filter_map(|s| s.virtualize(&vfs));
			output.extend(virtual_songs.map(CollectionFile::Song));
		}

		Ok(output)
	}

	pub fn flatten<P>(&self, virtual_path: P) -> Result<Vec<Song>, QueryError>
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

		let virtual_songs = real_songs.into_iter().filter_map(|s| s.virtualize(&vfs));
		Ok(virtual_songs.collect::<Vec<_>>())
	}

	pub fn get_random_albums(&self, count: i64) -> Result<Vec<Directory>, QueryError> {
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
			.filter_map(|d| d.virtualize(&vfs));
		Ok(virtual_directories.collect::<Vec<_>>())
	}

	pub fn get_recent_albums(&self, count: i64) -> Result<Vec<Directory>, QueryError> {
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
			.filter_map(|d| d.virtualize(&vfs));
		Ok(virtual_directories.collect::<Vec<_>>())
	}

	pub fn search(&self, query: &str) -> Result<Vec<CollectionFile>, QueryError> {
		let vfs = self.vfs_manager.get_vfs()?;
		let mut connection = self.db.connect()?;
		let like_test = format!("%{}%", query);
		let mut output = Vec::new();

		// Find dirs with matching path and parent not matching
		{
			use self::directories::dsl::*;
			let real_directories: Vec<Directory> = directories
				.filter(path.like(&like_test))
				.filter(parent.not_like(&like_test))
				.load(&mut connection)?;

			let virtual_directories = real_directories
				.into_iter()
				.filter_map(|d| d.virtualize(&vfs));

			output.extend(virtual_directories.map(CollectionFile::Directory));
		}

		// Find songs with matching title/album/artist and non-matching parent
		{
			use self::songs::dsl::*;
			let real_songs: Vec<Song> = songs
				.filter(
					path.like(&like_test)
						.or(title.like(&like_test))
						.or(album.like(&like_test))
						.or(artist.like(&like_test))
						.or(album_artist.like(&like_test)),
				)
				.filter(parent.not_like(&like_test))
				.load(&mut connection)?;

			let virtual_songs = real_songs.into_iter().filter_map(|d| d.virtualize(&vfs));

			output.extend(virtual_songs.map(CollectionFile::Song));
		}

		Ok(output)
	}

	pub fn get_song(&self, virtual_path: &Path) -> Result<Song, QueryError> {
		let vfs = self.vfs_manager.get_vfs()?;
		let mut connection = self.db.connect()?;

		let real_path = vfs.virtual_to_real(virtual_path)?;
		let real_path_string = real_path.as_path().to_string_lossy();

		use self::songs::dsl::*;
		let real_song: Song = songs
			.filter(path.eq(real_path_string))
			.get_result(&mut connection)?;

		match real_song.virtualize(&vfs) {
			Some(s) => Ok(s),
			None => Err(QueryError::SongNotFound(real_path)),
		}
	}
}
