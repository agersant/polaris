use rayon::prelude::*;
use sqlx::{QueryBuilder, Sqlite};
use std::path::Path;

use crate::app::vfs;
use crate::db::{self, DB};

const INDEX_BUILDING_CLEAN_BUFFER_SIZE: usize = 500; // Deletions in each transaction

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error(transparent)]
	Database(#[from] sqlx::Error),
	#[error(transparent)]
	DatabaseConnection(#[from] db::Error),
	#[error(transparent)]
	ThreadPoolBuilder(#[from] rayon::ThreadPoolBuildError),
	#[error(transparent)]
	Vfs(#[from] vfs::Error),
}

pub struct Cleaner {
	db: DB,
	vfs_manager: vfs::Manager,
}

impl Cleaner {
	pub fn new(db: DB, vfs_manager: vfs::Manager) -> Self {
		Self { db, vfs_manager }
	}

	pub async fn clean(&self) -> Result<(), Error> {
		let vfs = self.vfs_manager.get_vfs().await?;

		let (all_directories, all_songs) = {
			let mut connection = self.db.connect().await?;

			let directories = sqlx::query_scalar!("SELECT path FROM directories")
				.fetch_all(connection.as_mut())
				.await
				.unwrap();

			let songs = sqlx::query_scalar!("SELECT path FROM songs")
				.fetch_all(connection.as_mut())
				.await
				.unwrap();

			(directories, songs)
		};

		let list_missing_directories = || {
			all_directories
				.par_iter()
				.filter(|ref directory_path| {
					let path = Path::new(&directory_path);
					!path.exists() || vfs.real_to_virtual(path).is_err()
				})
				.collect::<Vec<_>>()
		};

		let list_missing_songs = || {
			all_songs
				.par_iter()
				.filter(|ref song_path| {
					let path = Path::new(&song_path);
					!path.exists() || vfs.real_to_virtual(path).is_err()
				})
				.collect::<Vec<_>>()
		};

		let thread_pool = rayon::ThreadPoolBuilder::new().build()?;
		let (missing_directories, missing_songs) =
			thread_pool.join(list_missing_directories, list_missing_songs);

		{
			let mut connection = self.db.connect().await?;

			for chunk in missing_directories[..].chunks(INDEX_BUILDING_CLEAN_BUFFER_SIZE) {
				QueryBuilder::<Sqlite>::new("DELETE FROM directories WHERE path IN ")
					.push_tuples(chunk, |mut b, path| {
						b.push_bind(path);
					})
					.build()
					.execute(connection.as_mut())
					.await?;
			}

			for chunk in missing_songs[..].chunks(INDEX_BUILDING_CLEAN_BUFFER_SIZE) {
				QueryBuilder::<Sqlite>::new("DELETE FROM songs WHERE path IN ")
					.push_tuples(chunk, |mut b, path| {
						b.push_bind(path);
					})
					.build()
					.execute(connection.as_mut())
					.await?;
			}
		}

		Ok(())
	}
}
