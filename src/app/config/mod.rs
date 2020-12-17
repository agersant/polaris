use core::ops::Deref;
use regex::Regex;
use serde::Deserialize;
use std::io::Read;
use std::path::{self, PathBuf};

use crate::app::{settings, user};

mod error;
mod manager;
#[cfg(test)]
mod test;

pub use error::*;
pub use manager::*;

#[derive(Default, Deserialize)]
pub struct Config {
	pub settings: Option<settings::NewSettings>,
	pub users: Option<Vec<user::NewUser>>,
}

impl Config {
	pub fn from_path(path: &path::Path) -> anyhow::Result<Config> {
		let mut config_file = std::fs::File::open(path)?;
		let mut config_file_content = String::new();
		config_file.read_to_string(&mut config_file_content)?;
		let mut config = toml::de::from_str::<Self>(&config_file_content)?;
		config.clean_paths()?;
		Ok(config)
	}

	// TODO find a better home for this?
	fn clean_paths(&mut self) -> anyhow::Result<()> {
		if let Some(ref mut settings) = self.settings {
			if let Some(ref mut mount_dirs) = settings.mount_dirs {
				for mount_dir in mount_dirs {
					match Self::clean_path_string(&mount_dir.source).to_str() {
						Some(p) => mount_dir.source = p.to_owned(),
						_ => anyhow::bail!("Bad mount directory path"),
					}
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
