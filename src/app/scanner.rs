use log::{error, info};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Notify;

use crate::app::{settings, vfs};
use crate::db::DB;

mod cleaner;
mod collector;
mod inserter;
mod metadata;
#[cfg(test)]
mod test;
mod traverser;
mod types;

pub use self::types::*;

#[derive(Clone)]
pub struct Scanner {
	db: DB,
	vfs_manager: vfs::Manager,
	settings_manager: settings::Manager,
	pending_scan: Arc<Notify>,
}

impl Scanner {
	pub fn new(db: DB, vfs_manager: vfs::Manager, settings_manager: settings::Manager) -> Self {
		let scanner = Self {
			db,
			vfs_manager,
			settings_manager,
			pending_scan: Arc::new(Notify::new()),
		};

		tokio::spawn({
			let scanner = scanner.clone();
			async move {
				loop {
					scanner.pending_scan.notified().await;
					if let Err(e) = scanner.scan().await {
						error!("Error while updating index: {}", e);
					}
				}
			}
		});

		scanner
	}

	pub fn trigger_scan(&self) {
		self.pending_scan.notify_one();
	}

	pub fn begin_periodic_scans(&self) {
		tokio::spawn({
			let index = self.clone();
			async move {
				loop {
					index.trigger_scan();
					let sleep_duration = index
						.settings_manager
						.get_index_sleep_duration()
						.await
						.unwrap_or_else(|e| {
							error!("Could not retrieve index sleep duration: {}", e);
							Duration::from_secs(1800)
						});
					tokio::time::sleep(sleep_duration).await;
				}
			}
		});
	}

	pub async fn scan(&self) -> Result<(), types::Error> {
		let start = Instant::now();
		info!("Beginning library index update");

		let album_art_pattern = self
			.settings_manager
			.get_index_album_art_pattern()
			.await
			.ok();

		let cleaner = cleaner::Cleaner::new(self.db.clone(), self.vfs_manager.clone());
		cleaner.clean().await?;

		let (insert_sender, insert_receiver) = tokio::sync::mpsc::unbounded_channel();
		let insertion = tokio::spawn({
			let db = self.db.clone();
			async {
				let mut inserter = inserter::Inserter::new(db, insert_receiver);
				inserter.insert().await;
			}
		});

		let (collect_sender, collect_receiver) = crossbeam_channel::unbounded();
		let collection = tokio::task::spawn_blocking(|| {
			let collector =
				collector::Collector::new(collect_receiver, insert_sender, album_art_pattern);
			collector.collect();
		});

		let vfs = self.vfs_manager.get_vfs().await?;
		let traversal = tokio::task::spawn_blocking(move || {
			let mounts = vfs.mounts();
			let traverser = traverser::Traverser::new(collect_sender);
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
