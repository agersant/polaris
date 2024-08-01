use serde::{Deserialize, Serialize};

use crate::app::{
	collection::{self},
	config, ddns, settings, thumbnail, user, vfs,
};
use std::{convert::From, path::PathBuf};

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Version {
	pub major: i32,
	pub minor: i32,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct InitialSetup {
	pub has_any_users: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Credentials {
	pub username: String,
	pub password: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Authorization {
	pub username: String,
	pub token: String,
	pub is_admin: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AuthQueryParameters {
	pub auth_token: String,
}

#[derive(Serialize, Deserialize)]
pub struct ThumbnailOptions {
	pub size: Option<ThumbnailSize>,
	pub pad: Option<bool>,
}

impl From<ThumbnailOptions> for thumbnail::Options {
	fn from(dto: ThumbnailOptions) -> Self {
		let mut options = thumbnail::Options::default();
		options.max_dimension = dto.size.map_or(options.max_dimension, Into::into);
		options.pad_to_square = dto.pad.unwrap_or(options.pad_to_square);
		options
	}
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThumbnailSize {
	Small,
	Large,
	Native,
}

#[allow(clippy::from_over_into)]
impl Into<Option<u32>> for ThumbnailSize {
	fn into(self) -> Option<u32> {
		match self {
			Self::Small => Some(400),
			Self::Large => Some(1200),
			Self::Native => None,
		}
	}
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListPlaylistsEntry {
	pub name: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SavePlaylistInput {
	pub tracks: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct LastFMLink {
	pub auth_token: String, // user::AuthToken emitted by Polaris, valid for LastFMLink scope
	pub token: String,      // LastFM token for use in scrobble calls
	pub content: String,    // Payload to send back to client after successful link
}

#[derive(Serialize, Deserialize)]
pub struct LastFMLinkToken {
	pub value: String,
}

#[derive(Serialize, Deserialize)]
pub struct User {
	pub name: String,
	pub is_admin: bool,
}

impl From<user::User> for User {
	fn from(u: user::User) -> Self {
		Self {
			name: u.name,
			is_admin: u.admin != 0,
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserUpdate {
	pub new_password: Option<String>,
	pub new_is_admin: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct DDNSConfig {
	pub host: String,
	pub username: String,
	pub password: String,
}

impl From<DDNSConfig> for ddns::Config {
	fn from(c: DDNSConfig) -> Self {
		Self {
			ddns_host: c.host,
			ddns_username: c.username,
			ddns_password: c.password,
		}
	}
}

impl From<ddns::Config> for DDNSConfig {
	fn from(c: ddns::Config) -> Self {
		Self {
			host: c.ddns_host,
			username: c.ddns_username,
			password: c.ddns_password,
		}
	}
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct MountDir {
	pub source: String,
	pub name: String,
}

impl From<MountDir> for vfs::MountDir {
	fn from(m: MountDir) -> Self {
		Self {
			name: m.name,
			source: m.source,
		}
	}
}

impl From<vfs::MountDir> for MountDir {
	fn from(m: vfs::MountDir) -> Self {
		Self {
			name: m.name,
			source: m.source,
		}
	}
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
	pub settings: Option<NewSettings>,
	pub users: Option<Vec<NewUser>>,
	pub mount_dirs: Option<Vec<MountDir>>,
	pub ydns: Option<DDNSConfig>,
}

impl From<Config> for config::Config {
	fn from(s: Config) -> Self {
		Self {
			settings: s.settings.map(|s| s.into()),
			mount_dirs: s
				.mount_dirs
				.map(|v| v.into_iter().map(|m| m.into()).collect()),
			users: s.users.map(|v| v.into_iter().map(|u| u.into()).collect()),
			ydns: s.ydns.map(|c| c.into()),
		}
	}
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewSettings {
	pub album_art_pattern: Option<String>,
	pub reindex_every_n_seconds: Option<i64>,
}

impl From<NewSettings> for settings::NewSettings {
	fn from(s: NewSettings) -> Self {
		Self {
			album_art_pattern: s.album_art_pattern,
			reindex_every_n_seconds: s.reindex_every_n_seconds,
		}
	}
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
	pub album_art_pattern: String,
	pub reindex_every_n_seconds: i64,
}

impl From<settings::Settings> for Settings {
	fn from(s: settings::Settings) -> Self {
		Self {
			album_art_pattern: s.index_album_art_pattern,
			reindex_every_n_seconds: s.index_sleep_duration_seconds,
		}
	}
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CollectionFile {
	Directory(Directory),
	Song(Song),
}

impl From<collection::File> for CollectionFile {
	fn from(f: collection::File) -> Self {
		match f {
			collection::File::Directory(d) => Self::Directory(Directory {
				path: d,
				artist: None,
				year: None,
				album: None,
				artwork: None,
			}),
			collection::File::Song(s) => Self::Song(Song {
				path: s,
				track_number: None,
				disc_number: None,
				title: None,
				artist: None,
				album_artist: None,
				year: None,
				album: None,
				artwork: None,
				duration: None,
				lyricist: None,
				composer: None,
				genre: None,
				label: None,
			}),
		}
	}
}

trait VecExt {
	fn to_v7_string(&self) -> Option<String>;
}

impl VecExt for Vec<String> {
	fn to_v7_string(&self) -> Option<String> {
		if self.is_empty() {
			None
		} else {
			Some(self.join(""))
		}
	}
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Song {
	pub path: PathBuf,
	pub track_number: Option<i64>,
	pub disc_number: Option<i64>,
	pub title: Option<String>,
	pub artist: Option<String>,
	pub album_artist: Option<String>,
	pub year: Option<i64>,
	pub album: Option<String>,
	pub artwork: Option<PathBuf>,
	pub duration: Option<i64>,
	pub lyricist: Option<String>,
	pub composer: Option<String>,
	pub genre: Option<String>,
	pub label: Option<String>,
}

impl From<collection::SongKey> for Song {
	fn from(song_key: collection::SongKey) -> Self {
		Self {
			path: song_key.virtual_path,
			track_number: None,
			disc_number: None,
			title: None,
			artist: None,
			album_artist: None,
			year: None,
			album: None,
			artwork: None,
			duration: None,
			lyricist: None,
			composer: None,
			genre: None,
			label: None,
		}
	}
}

impl From<collection::Song> for Song {
	fn from(s: collection::Song) -> Self {
		Self {
			path: s.virtual_path,
			track_number: s.track_number,
			disc_number: s.disc_number,
			title: s.title,
			artist: s.artists.first().cloned(),
			album_artist: s.album_artists.to_v7_string(),
			year: s.year,
			album: s.album,
			artwork: s.artwork,
			duration: s.duration,
			lyricist: s.lyricists.to_v7_string(),
			composer: s.composers.to_v7_string(),
			genre: s.genres.to_v7_string(),
			label: s.labels.to_v7_string(),
		}
	}
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Directory {
	pub path: PathBuf,
	pub artist: Option<String>,
	pub year: Option<i64>,
	pub album: Option<String>,
	pub artwork: Option<PathBuf>,
}

impl From<collection::Album> for Directory {
	fn from(album: collection::Album) -> Self {
		let path = album
			.songs
			.first()
			.map(|s| s.virtual_parent.clone())
			.unwrap_or_default();

		Self {
			path,
			artist: match album.artists.is_empty() {
				true => None,
				false => Some(album.artists.join("")),
			},
			year: album.year,
			album: album.name,
			artwork: album.artwork,
		}
	}
}
