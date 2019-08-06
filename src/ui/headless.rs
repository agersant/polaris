use log::info;
use std::thread;
use std::time;

pub fn run() {
	info!("Starting up UI (headless)");
	loop {
		thread::sleep(time::Duration::from_secs(10));
	}
}
