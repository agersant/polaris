use std::{io::Read, path::Path};

use serde::{Deserialize, Serialize};

use crate::app::Error;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct User {
	pub name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub admin: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub initial_password: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub hashed_password: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct MountDir {
	pub source: String,
	pub name: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Config {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub reindex_every_n_seconds: Option<u64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub album_art_pattern: Option<String>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub mount_dirs: Vec<MountDir>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ddns_url: Option<String>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub users: Vec<User>,
}

impl Config {
	pub fn from_path(path: &Path) -> Result<Self, Error> {
		let mut config_file =
			std::fs::File::open(path).map_err(|e| Error::Io(path.to_owned(), e))?;
		let mut config_file_content = String::new();
		config_file
			.read_to_string(&mut config_file_content)
			.map_err(|e| Error::Io(path.to_owned(), e))?;
		let config = toml::de::from_str::<Self>(&config_file_content)?;
		Ok(config)
	}
}
