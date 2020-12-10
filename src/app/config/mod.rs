use crate::app::{ddns, vfs};
use core::ops::Deref;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{self, PathBuf};

mod error;
mod manager;
#[cfg(test)]
mod test;

pub use error::*;
pub use manager::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConfigUser {
	pub name: String,
	pub password: String,
	pub admin: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Config {
	pub album_art_pattern: Option<String>,
	pub reindex_every_n_seconds: Option<i32>,
	pub mount_dirs: Option<Vec<vfs::MountPoint>>,
	pub users: Option<Vec<ConfigUser>>,
	pub ydns: Option<ddns::Config>,
}

impl Config {
	pub fn from_path(path: &path::Path) -> anyhow::Result<Config> {
		let mut config_file = std::fs::File::open(path)?;
		let mut config_file_content = String::new();
		config_file.read_to_string(&mut config_file_content)?;
		let mut config = toml::de::from_str::<Config>(&config_file_content)?;
		config.clean_paths()?;
		Ok(config)
	}

	fn clean_paths(&mut self) -> anyhow::Result<()> {
		if let Some(ref mut mount_dirs) = self.mount_dirs {
			for mount_dir in mount_dirs {
				match Self::clean_path_string(&mount_dir.source).to_str() {
					Some(p) => mount_dir.source = p.to_owned(),
					_ => anyhow::bail!("Bad mount directory path"),
				}
			}
		}
		Ok(())
	}

	fn clean_path_string(path_string: &str) -> PathBuf {
		let separator_regex = Regex::new(r"\\|/").unwrap();
		let mut correct_separator = String::new();
		correct_separator.push(path::MAIN_SEPARATOR);
		let path_string = separator_regex.replace_all(path_string, correct_separator.as_str());
		path::Path::new(path_string.deref()).iter().collect()
	}
}

#[derive(Debug, Queryable)]
pub struct MiscSettings {
	id: i32,
	pub auth_secret: Vec<u8>,
	pub index_sleep_duration_seconds: i32,
	pub index_album_art_pattern: String,
}
