use anyhow::*;
use diesel;
use diesel::prelude::*;
#[cfg(feature = "profile-index")]
use flame;
use log::{error, info};
use regex::Regex;
use std::time;

use crate::config::MiscSettings;
use crate::db::{misc_settings, DB};
use crate::vfs::VFSSource;

mod cleaner;
mod collector;
mod inserter;
mod traverser;

use cleaner::Cleaner;
use collector::Collector;
use inserter::Inserter;
use traverser::Traverser;

pub fn update(db: &DB) -> Result<()> {
	let start = time::Instant::now();
	info!("Beginning library index update");

	let album_art_pattern = {
		let connection = db.connect()?;
		let settings: MiscSettings = misc_settings::table.get_result(&connection)?;
		Regex::new(&settings.index_album_art_pattern)?
	};

	let cleaner = Cleaner::new(db.clone());
	cleaner.clean()?;

	let (insert_sender, insert_receiver) = crossbeam_channel::unbounded();
	let inserter_db = db.clone();
	let insertion_thread = std::thread::spawn(move || {
		let mut inserter = Inserter::new(inserter_db, insert_receiver);
		inserter.insert();
	});

	let (collect_sender, collect_receiver) = crossbeam_channel::unbounded();
	let collector_thread = std::thread::spawn(move || {
		let collector = Collector::new(collect_receiver, insert_sender, album_art_pattern);
		collector.collect();
	});

	let vfs = db.get_vfs()?;
	let traverser_thread = std::thread::spawn(move || {
		#[cfg(feature = "profile-index")]
		flame::clear();

		let mount_points = vfs.get_mount_points();
		let traverser = Traverser::new(collect_sender);
		traverser.traverse(mount_points.values().map(|p| p.clone()).collect());

		#[cfg(feature = "profile-index")]
		flame::dump_html(&mut std::fs::File::create("profile-index.html").unwrap()).unwrap();
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
