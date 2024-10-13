use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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
	pub source: PathBuf,
	pub name: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Config {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub album_art_pattern: Option<String>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub mount_dirs: Vec<MountDir>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ddns_update_url: Option<String>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub users: Vec<User>,
}
