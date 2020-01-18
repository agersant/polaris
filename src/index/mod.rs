use anyhow::*;
use core::ops::Deref;
use diesel;
use diesel::prelude::*;
#[cfg(feature = "profile-index")]
use flame;
use log::{error, info};
use std::path::Path;
use std::sync::mpsc::*;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;

use crate::config::MiscSettings;
use crate::db::{directories, misc_settings, songs, DB};
use crate::vfs::VFS;

mod populate;
mod query;
#[cfg(test)]
mod test;
mod types;

pub use self::populate::*;
pub use self::query::*;
pub use self::types::*;

enum Command {
	REINDEX,
	EXIT,
}

struct CommandReceiver {
	receiver: Receiver<Command>,
}

impl CommandReceiver {
	fn new(receiver: Receiver<Command>) -> CommandReceiver {
		CommandReceiver { receiver }
	}
}

pub struct CommandSender {
	sender: Mutex<Sender<Command>>,
}

impl CommandSender {
	fn new(sender: Sender<Command>) -> CommandSender {
		CommandSender {
			sender: Mutex::new(sender),
		}
	}

	pub fn trigger_reindex(&self) -> Result<()> {
		let sender = self.sender.lock().unwrap();
		match sender.send(Command::REINDEX) {
			Ok(_) => Ok(()),
			Err(_) => bail!("Trigger reindex channel error"),
		}
	}

	#[allow(dead_code)]
	pub fn exit(&self) -> Result<()> {
		let sender = self.sender.lock().unwrap();
		match sender.send(Command::EXIT) {
			Ok(_) => Ok(()),
			Err(_) => bail!("Index exit channel error"),
		}
	}
}

pub fn init(db: DB) -> Arc<CommandSender> {
	let (index_sender, index_receiver) = channel();
	let command_sender = Arc::new(CommandSender::new(index_sender));
	let command_receiver = CommandReceiver::new(index_receiver);

	// Start update loop
	std::thread::spawn(move || {
		update_loop(&db, &command_receiver);
	});

	command_sender
}

pub fn update(db: &DB) -> Result<()> {
	let start = time::Instant::now();
	info!("Beginning library index update");
	clean(db)?;
	populate(db)?;
	info!(
		"Library index update took {} seconds",
		start.elapsed().as_secs()
	);
	#[cfg(feature = "profile-index")]
	flame::dump_html(&mut fs::File::create("index-flame-graph.html").unwrap()).unwrap();
	Ok(())
}

fn update_loop(db: &DB, command_buffer: &CommandReceiver) {
	loop {
		// Wait for a command
		if command_buffer.receiver.recv().is_err() {
			return;
		}

		// Flush the buffer to ignore spammy requests
		loop {
			match command_buffer.receiver.try_recv() {
				Err(TryRecvError::Disconnected) => return,
				Ok(Command::EXIT) => return,
				Err(TryRecvError::Empty) => break,
				Ok(_) => (),
			}
		}

		// Do the update
		if let Err(e) = update(db) {
			error!("Error while updating index: {}", e);
		}
	}
}

pub fn self_trigger(db: &DB, command_buffer: &Arc<CommandSender>) {
	loop {
		{
			let command_buffer = command_buffer.deref();
			if let Err(e) = command_buffer.trigger_reindex() {
				error!("Error while writing to index command buffer: {}", e);
				return;
			}
		}
		let sleep_duration = {
			let connection = db.connect();
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
		thread::sleep(time::Duration::from_secs(sleep_duration as u64));
	}
}
