use rayon::prelude::*;
use sqlx::{QueryBuilder, Sqlite};
use std::path::Path;

use crate::app::{collection, vfs};
use crate::db::DB;

#[derive(Clone)]
pub struct Cleaner {
	db: DB,
	vfs_manager: vfs::Manager,
}

impl Cleaner {
	const BUFFER_SIZE: usize = 500; // Deletions in each transaction

	pub fn new(db: DB, vfs_manager: vfs::Manager) -> Self {
		Self { db, vfs_manager }
	}

	pub async fn clean(&self) -> Result<(), collection::Error> {
		tokio::try_join!(self.clean_songs(), self.clean_directories())?;
		Ok(())
	}

	pub async fn clean_directories(&self) -> Result<(), collection::Error> {
		let directories = {
			let mut connection = self.db.connect().await?;
			sqlx::query!("SELECT path, virtual_path FROM directories")
				.fetch_all(connection.as_mut())
				.await?
		};

		let vfs = self.vfs_manager.get_vfs().await?;
		let missing_directories = tokio::task::spawn_blocking(move || {
			directories
				.into_par_iter()
				.filter(|d| !vfs.exists(&d.virtual_path) || !Path::new(&d.path).exists())
				.map(|d| d.virtual_path)
				.collect::<Vec<_>>()
		})
		.await?;

		let mut connection = self.db.connect().await?;
		for chunk in missing_directories[..].chunks(Self::BUFFER_SIZE) {
			QueryBuilder::<Sqlite>::new("DELETE FROM directories WHERE virtual_path IN ")
				.push_tuples(chunk, |mut b, virtual_path| {
					b.push_bind(virtual_path);
				})
				.build()
				.execute(connection.as_mut())
				.await?;
		}

		Ok(())
	}

	pub async fn clean_songs(&self) -> Result<(), collection::Error> {
		let songs = {
			let mut connection = self.db.connect().await?;
			sqlx::query!("SELECT path, virtual_path FROM songs")
				.fetch_all(connection.as_mut())
				.await?
		};

		let vfs = self.vfs_manager.get_vfs().await?;
		let deleted_songs = tokio::task::spawn_blocking(move || {
			songs
				.into_par_iter()
				.filter(|s| !vfs.exists(&s.virtual_path) || !Path::new(&s.path).exists())
				.map(|s| s.virtual_path)
				.collect::<Vec<_>>()
		})
		.await?;

		for chunk in deleted_songs[..].chunks(Cleaner::BUFFER_SIZE) {
			let mut connection = self.db.connect().await?;
			QueryBuilder::<Sqlite>::new("DELETE FROM songs WHERE virtual_path IN ")
				.push_tuples(chunk, |mut b, virtual_path| {
					b.push_bind(virtual_path);
				})
				.build()
				.execute(connection.as_mut())
				.await?;
		}

		Ok(())
	}
}
