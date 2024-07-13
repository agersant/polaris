use log::error;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;

use crate::app::{settings, vfs};
use crate::db::DB;

mod metadata;
mod query;
#[cfg(test)]
mod test;
mod types;
mod update;

pub use self::query::*;
pub use self::types::*;

#[derive(Clone)]
pub struct Index {
	db: DB,
	vfs_manager: vfs::Manager,
	settings_manager: settings::Manager,
	pending_reindex: Arc<Notify>,
}

impl Index {
	pub fn new(db: DB, vfs_manager: vfs::Manager, settings_manager: settings::Manager) -> Self {
		let index = Self {
			db,
			vfs_manager,
			settings_manager,
			pending_reindex: Arc::new(Notify::new()),
		};

		tokio::spawn({
			let index = index.clone();
			async move {
				loop {
					index.pending_reindex.notified().await;
					if let Err(e) = index.update().await {
						error!("Error while updating index: {}", e);
					}
				}
			}
		});

		index
	}

	pub fn trigger_reindex(&self) {
		self.pending_reindex.notify_one();
	}

	pub fn begin_periodic_updates(&self) {
		tokio::spawn({
			let index = self.clone();
			async move {
				loop {
					index.trigger_reindex();
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
}
