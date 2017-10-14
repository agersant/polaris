use std::time;
use std::thread;

pub fn run() {
	info!("Starting up UI (headless)");
	loop {
		thread::sleep(time::Duration::from_secs(10));
	}
}
