use serde::{Deserialize, Serialize};

use crate::app::{config, ddns, settings, user, vfs};

pub const API_MAJOR_VERSION: i32 = 6;
pub const API_MINOR_VERSION: i32 = 0;
pub const COOKIE_SESSION: &str = "session";
pub const COOKIE_USERNAME: &str = "username";
pub const COOKIE_ADMIN: &str = "admin";

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct Version {
	pub major: i32,
	pub minor: i32,
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct InitialSetup {
	pub has_any_users: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AuthCredentials {
	pub username: String,
	pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct ThumbnailOptions {
	pub pad: Option<bool>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ListPlaylistsEntry {
	pub name: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SavePlaylistInput {
	pub tracks: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct LastFMLink {
	pub token: String,
	pub content: String,
}

#[derive(Serialize, Deserialize)]
pub struct User {
	pub username: String,
	pub is_admin: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NewUser {
	pub name: String,
	pub password: String,
	pub admin: bool,
}

impl From<NewUser> for user::NewUser {
	fn from(u: NewUser) -> Self {
		Self {
			name: u.name,
			password: u.password,
			admin: u.admin,
		}
	}
}

#[derive(Serialize, Deserialize)]
pub struct Users {
	users: Vec<User>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DDNSConfig {
	pub host: String,
	pub username: String,
	pub password: String,
}

impl From<DDNSConfig> for ddns::Config {
	fn from(c: DDNSConfig) -> Self {
		Self {
			host: c.host,
			username: c.username,
			password: c.password,
		}
	}
}

impl From<ddns::Config> for DDNSConfig {
	fn from(c: ddns::Config) -> Self {
		Self {
			host: c.host,
			username: c.username,
			password: c.password,
		}
	}
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MountPoint {
	pub source: String,
	pub name: String,
}

impl From<MountPoint> for vfs::MountPoint {
	fn from(m: MountPoint) -> Self {
		Self {
			name: m.name,
			source: m.source,
		}
	}
}

impl From<vfs::MountPoint> for MountPoint {
	fn from(m: vfs::MountPoint) -> Self {
		Self {
			name: m.name,
			source: m.source,
		}
	}
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Config {
	pub settings: Option<NewSettings>,
	pub users: Option<Vec<NewUser>>,
	pub ydns: Option<DDNSConfig>,
}

impl From<Config> for config::Config {
	fn from(s: Config) -> Self {
		Self {
			settings: s.settings.map(|s| s.into()),
			users: s.users.map(|v| v.into_iter().map(|u| u.into()).collect()),
		}
	}
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NewSettings {
	pub album_art_pattern: Option<String>,
	pub reindex_every_n_seconds: Option<i32>,
	pub mount_dirs: Option<Vec<MountPoint>>,
	pub ydns: Option<DDNSConfig>,
}

impl From<NewSettings> for settings::NewSettings {
	fn from(s: NewSettings) -> Self {
		Self {
			album_art_pattern: s.album_art_pattern,
			reindex_every_n_seconds: s.reindex_every_n_seconds,
			mount_dirs: s
				.mount_dirs
				.map(|v| Some(v.into_iter().map(|m| m.into()).collect()))
				.unwrap_or(None),
			ydns: s.ydns.map(|c| c.into()),
		}
	}
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Settings {
	pub album_art_pattern: String,
	pub reindex_every_n_seconds: i32,
	pub mount_dirs: Vec<MountPoint>,
	pub ydns: Option<DDNSConfig>,
}

impl From<settings::Settings> for Settings {
	fn from(s: settings::Settings) -> Self {
		Self {
			album_art_pattern: s.album_art_pattern,
			reindex_every_n_seconds: s.reindex_every_n_seconds,
			mount_dirs: s.mount_dirs.into_iter().map(|m| m.into()).collect(),
			ydns: s.ydns.map(|c| c.into()),
		}
	}
}

// TODO: Preferences, CollectionFile, Song and Directory should have dto types
