use std::time;
use std::thread;

pub fn run() {
	println!("Starting up UI (headless)");
	loop {
		thread::sleep(time::Duration::from_secs(10));
	}
}
