use serde::Deserialize;
use std::io::Read;
use std::path;

use crate::app::{ddns, settings, user, vfs};

mod error;
mod manager;
#[cfg(test)]
mod test;

pub use error::*;
pub use manager::*;

#[derive(Default, Deserialize)]
pub struct Config {
	pub settings: Option<settings::NewSettings>,
	pub mount_dirs: Option<Vec<vfs::MountDir>>,
	pub ydns: Option<ddns::Config>,
	pub users: Option<Vec<user::NewUser>>,
}

impl Config {
	pub fn from_path(path: &path::Path) -> anyhow::Result<Config> {
		let mut config_file = std::fs::File::open(path)?;
		let mut config_file_content = String::new();
		config_file.read_to_string(&mut config_file_content)?;
		let config = toml::de::from_str::<Self>(&config_file_content)?;
		Ok(config)
	}
}
