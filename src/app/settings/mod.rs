use serde::Deserialize;

mod error;
mod manager;

use crate::app::{ddns, vfs};

pub use error::*;
pub use manager::*;

#[derive(Clone)]
pub struct AuthSecret {
	pub key: [u8; 32],
}

#[derive(Debug, Queryable)]
struct MiscSettings {
	id: i32,
	auth_secret: Vec<u8>,
	index_sleep_duration_seconds: i32,
	index_album_art_pattern: String,
}

#[derive(Debug)]
pub struct Settings {
	auth_secret: Vec<u8>,
	pub index_sleep_duration_seconds: i32,
	pub index_album_art_pattern: String,
	pub mount_dirs: Vec<vfs::MountPoint>,
	pub ydns: Option<ddns::Config>,
}

#[derive(Debug, Deserialize)]
pub struct NewSettings {
	pub index_sleep_duration_seconds: Option<i32>,
	pub index_album_art_pattern: Option<String>,
	pub mount_dirs: Option<Vec<vfs::MountPoint>>,
	pub ydns: Option<ddns::Config>,
}
