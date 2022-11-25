use log::{error, info};
use std::time;

mod cleaner;
mod collector;
mod inserter;
mod traverser;

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
	Database(#[from] diesel::result::Error),
	#[error(transparent)]
	DatabaseConnection(#[from] db::Error),
	#[error(transparent)]
	Vfs(#[from] vfs::Error),
}

impl Index {
	pub fn update(&self) -> Result<(), Error> {
		let start = time::Instant::now();
		info!("Beginning library index update");

		let album_art_pattern = self.settings_manager.get_index_album_art_pattern().ok();

		let cleaner = Cleaner::new(self.db.clone(), self.vfs_manager.clone());
		cleaner.clean()?;

		let (insert_sender, insert_receiver) = crossbeam_channel::unbounded();
		let inserter_db = self.db.clone();
		let insertion_thread = std::thread::spawn(move || {
			let mut inserter = Inserter::new(inserter_db, insert_receiver);
			inserter.insert();
		});

		let (collect_sender, collect_receiver) = crossbeam_channel::unbounded();
		let collector_thread = std::thread::spawn(move || {
			let collector = Collector::new(collect_receiver, insert_sender, album_art_pattern);
			collector.collect();
		});

		let vfs = self.vfs_manager.get_vfs()?;
		let traverser_thread = std::thread::spawn(move || {
			let mounts = vfs.mounts();
			let traverser = Traverser::new(collect_sender);
			traverser.traverse(mounts.iter().map(|p| p.source.clone()).collect());
		});

		if let Err(e) = traverser_thread.join() {
			error!("Error joining on traverser thread: {:?}", e);
		}

		if let Err(e) = collector_thread.join() {
			error!("Error joining on collector thread: {:?}", e);
		}

		if let Err(e) = insertion_thread.join() {
			error!("Error joining on inserter thread: {:?}", e);
		}

		info!(
			"Library index update took {} seconds",
			start.elapsed().as_millis() as f32 / 1000.0
		);

		Ok(())
	}
}
