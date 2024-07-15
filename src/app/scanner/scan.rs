use log::{error, info};
use std::time;

use crate::app::index::Index;
use crate::app::vfs;
use crate::db;

use cleaner::Cleaner;
use collector::Collector;
use inserter::Inserter;
use traverser::Traverser;

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error(transparent)]
	IndexClean(#[from] cleaner::Error),
	#[error(transparent)]
	Database(#[from] sqlx::Error),
	#[error(transparent)]
	DatabaseConnection(#[from] db::Error),
	#[error(transparent)]
	Vfs(#[from] vfs::Error),
}

impl Scanner {
	pub async fn scan(&self) -> Result<(), Error> {
		let start = time::Instant::now();
		info!("Beginning library index update");

		let album_art_pattern = self
			.settings_manager
			.get_index_album_art_pattern()
			.await
			.ok();

		let cleaner = Cleaner::new(self.db.clone(), self.vfs_manager.clone());
		cleaner.clean().await?;

		let (insert_sender, insert_receiver) = tokio::sync::mpsc::unbounded_channel();
		let insertion = tokio::spawn({
			let db = self.db.clone();
			async {
				let mut inserter = Inserter::new(db, insert_receiver);
				inserter.insert().await;
			}
		});

		let (collect_sender, collect_receiver) = crossbeam_channel::unbounded();
		let collection = tokio::task::spawn_blocking(|| {
			let collector = Collector::new(collect_receiver, insert_sender, album_art_pattern);
			collector.collect();
		});

		let vfs = self.vfs_manager.get_vfs().await?;
		let traversal = tokio::task::spawn_blocking(move || {
			let mounts = vfs.mounts();
			let traverser = Traverser::new(collect_sender);
			traverser.traverse(mounts.iter().map(|p| p.source.clone()).collect());
		});

		traversal.await.unwrap();
		collection.await.unwrap();
		insertion.await.unwrap();

		info!(
			"Library index update took {} seconds",
			start.elapsed().as_millis() as f32 / 1000.0
		);

		Ok(())
	}
}
