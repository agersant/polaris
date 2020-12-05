use anyhow::*;
use diesel;
use diesel::prelude::*;
use log::error;
use std::sync::{Arc, Condvar, Mutex};
use std::time;

use crate::config::MiscSettings;
use crate::db::{misc_settings, DB};
use crate::vfs::VFS;

mod metadata;
mod query;
#[cfg(test)]
mod test;
mod types;
mod update;

pub use self::query::*;
pub use self::types::*;
pub use self::update::*;

#[derive(Clone)]
pub struct Index {
	db: DB,
	pending_reindex: Arc<(Mutex<bool>, Condvar)>,
}

impl Index {
	pub fn new(db: DB) -> Self {
		let index = Self {
			pending_reindex: Arc::new((Mutex::new(false), Condvar::new())),
			db,
		};

		let commands_index = index.clone();
		std::thread::spawn(move || {
			commands_index.process_commands();
		});

		index
	}

	pub fn trigger_reindex(&self) {
		let (lock, cvar) = &*self.pending_reindex;
		let mut pending_reindex = lock.lock().unwrap();
		*pending_reindex = true;
		cvar.notify_one();
	}

	pub fn begin_periodic_updates(&self) {
		let auto_index = self.clone();
		std::thread::spawn(move || {
			auto_index.automatic_reindex();
		});
	}

	fn process_commands(&self) {
		loop {
			{
				let (lock, cvar) = &*self.pending_reindex;
				let mut pending = lock.lock().unwrap();
				while !*pending {
					pending = cvar.wait(pending).unwrap();
				}
				*pending = false;
			}
			if let Err(e) = update(&self.db) {
				error!("Error while updating index: {}", e);
			}
		}
	}

	fn automatic_reindex(&self) {
		loop {
			self.trigger_reindex();
			let sleep_duration = {
				let connection = self.db.connect();
				connection
					.and_then(|c| {
						misc_settings::table
							.get_result(&c)
							.map_err(|e| Error::new(e))
					})
					.map(|s: MiscSettings| s.index_sleep_duration_seconds)
					.unwrap_or_else(|e| {
						error!("Could not retrieve index sleep duration: {}", e);
						1800
					})
			};
			std::thread::sleep(time::Duration::from_secs(sleep_duration as u64));
		}
	}
}
