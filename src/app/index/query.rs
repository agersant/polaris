use std::path::{Path, PathBuf};

use super::*;
use crate::db;

#[derive(thiserror::Error, Debug)]
pub enum QueryError {
	#[error(transparent)]
	Database(#[from] sqlx::Error),
	#[error(transparent)]
	DatabaseConnection(#[from] db::Error),
	#[error("Song was not found: `{0}`")]
	SongNotFound(PathBuf),
	#[error(transparent)]
	Vfs(#[from] vfs::Error),
}

impl Index {
	pub async fn browse<P>(&self, virtual_path: P) -> Result<Vec<CollectionFile>, QueryError>
	where
		P: AsRef<Path>,
	{
		let mut output = Vec::new();
		let vfs = self.vfs_manager.get_vfs().await?;
		let mut connection = self.db.connect().await?;

		if virtual_path.as_ref().components().count() == 0 {
			// Browse top-level
			let real_directories =
				sqlx::query_as!(Directory, "SELECT * FROM directories WHERE parent IS NULL")
					.fetch_all(connection.as_mut())
					.await?;
			let virtual_directories = real_directories
				.into_iter()
				.filter_map(|d| d.virtualize(&vfs));
			output.extend(virtual_directories.map(CollectionFile::Directory));
		} else {
			// Browse sub-directory
			let real_path = vfs.virtual_to_real(virtual_path)?;
			let real_path_string = real_path.as_path().to_string_lossy().into_owned();

			let real_directories = sqlx::query_as!(
				Directory,
				"SELECT * FROM directories WHERE parent = $1 ORDER BY path COLLATE NOCASE ASC",
				real_path_string
			)
			.fetch_all(connection.as_mut())
			.await?;

			let virtual_directories = real_directories
				.into_iter()
				.filter_map(|d| d.virtualize(&vfs));

			output.extend(virtual_directories.map(CollectionFile::Directory));

			let real_songs = sqlx::query_as!(
				Song,
				"SELECT * FROM songs WHERE parent = $1 ORDER BY path COLLATE NOCASE ASC",
				real_path_string
			)
			.fetch_all(connection.as_mut())
			.await?;

			let virtual_songs = real_songs.into_iter().filter_map(|s| s.virtualize(&vfs));
			output.extend(virtual_songs.map(CollectionFile::Song));
		}

		Ok(output)
	}

	pub async fn flatten<P>(&self, virtual_path: P) -> Result<Vec<Song>, QueryError>
	where
		P: AsRef<Path>,
	{
		let vfs = self.vfs_manager.get_vfs().await?;
		let mut connection = self.db.connect().await?;

		let real_songs = if virtual_path.as_ref().parent().is_some() {
			let real_path = vfs.virtual_to_real(virtual_path)?;
			let song_path_filter = {
				let mut path_buf = real_path;
				path_buf.push("%");
				path_buf.as_path().to_string_lossy().into_owned()
			};
			sqlx::query_as!(
				Song,
				"SELECT * FROM songs WHERE path LIKE $1 ORDER BY path COLLATE NOCASE ASC",
				song_path_filter
			)
			.fetch_all(connection.as_mut())
			.await?
		} else {
			sqlx::query_as!(Song, "SELECT * FROM songs ORDER BY path COLLATE NOCASE ASC")
				.fetch_all(connection.as_mut())
				.await?
		};

		let virtual_songs = real_songs.into_iter().filter_map(|s| s.virtualize(&vfs));
		Ok(virtual_songs.collect::<Vec<_>>())
	}

	pub async fn get_random_albums(&self, count: i64) -> Result<Vec<Directory>, QueryError> {
		let vfs = self.vfs_manager.get_vfs().await?;
		let mut connection = self.db.connect().await?;

		let real_directories = sqlx::query_as!(
			Directory,
			"SELECT * FROM directories WHERE album IS NOT NULL ORDER BY RANDOM() DESC LIMIT $1",
			count
		)
		.fetch_all(connection.as_mut())
		.await?;

		let virtual_directories = real_directories
			.into_iter()
			.filter_map(|d| d.virtualize(&vfs));
		Ok(virtual_directories.collect::<Vec<_>>())
	}

	pub async fn get_recent_albums(&self, count: i64) -> Result<Vec<Directory>, QueryError> {
		let vfs = self.vfs_manager.get_vfs().await?;
		let mut connection = self.db.connect().await?;

		let real_directories = sqlx::query_as!(
			Directory,
			"SELECT * FROM directories WHERE album IS NOT NULL ORDER BY date_added DESC LIMIT $1",
			count
		)
		.fetch_all(connection.as_mut())
		.await?;

		let virtual_directories = real_directories
			.into_iter()
			.filter_map(|d| d.virtualize(&vfs));
		Ok(virtual_directories.collect::<Vec<_>>())
	}

	pub async fn search(&self, query: &str) -> Result<Vec<CollectionFile>, QueryError> {
		let vfs = self.vfs_manager.get_vfs().await?;
		let mut connection = self.db.connect().await?;
		let like_test = format!("%{}%", query);
		let mut output = Vec::new();

		// Find dirs with matching path and parent not matching
		{
			let real_directories = sqlx::query_as!(
				Directory,
				"SELECT * FROM directories WHERE path LIKE $1 AND parent NOT LIKE $1",
				like_test
			)
			.fetch_all(connection.as_mut())
			.await?;

			let virtual_directories = real_directories
				.into_iter()
				.filter_map(|d| d.virtualize(&vfs));

			output.extend(virtual_directories.map(CollectionFile::Directory));
		}

		// Find songs with matching title/album/artist and non-matching parent
		{
			let real_songs = sqlx::query_as!(
				Song,
				r#"
				SELECT * FROM songs
				WHERE	(	path LIKE $1
						OR	title LIKE $1
						OR album LIKE $1
						OR artist LIKE $1
						OR album_artist LIKE $1
						)
					AND parent NOT LIKE $1
				"#,
				like_test
			)
			.fetch_all(connection.as_mut())
			.await?;

			let virtual_songs = real_songs.into_iter().filter_map(|d| d.virtualize(&vfs));

			output.extend(virtual_songs.map(CollectionFile::Song));
		}

		Ok(output)
	}

	pub async fn get_song(&self, virtual_path: &Path) -> Result<Song, QueryError> {
		let vfs = self.vfs_manager.get_vfs().await?;
		let mut connection = self.db.connect().await?;

		let real_path = vfs.virtual_to_real(virtual_path)?;
		let real_path_string = real_path.as_path().to_string_lossy();

		let real_song = sqlx::query_as!(
			Song,
			"SELECT * FROM songs WHERE path = $1",
			real_path_string
		)
		.fetch_one(connection.as_mut())
		.await?;

		match real_song.virtualize(&vfs) {
			Some(s) => Ok(s),
			None => Err(QueryError::SongNotFound(real_path)),
		}
	}
}
