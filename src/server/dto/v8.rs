use serde::{Deserialize, Serialize};

use crate::app::{config, ddns, index, settings, thumbnail, user, vfs};
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
	pub tracks: Vec<std::path::PathBuf>,
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
pub struct Song {
	pub path: PathBuf,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub track_number: Option<i64>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub disc_number: Option<i64>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub title: Option<String>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub artists: Vec<String>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub album_artists: Vec<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub year: Option<i64>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub album: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub artwork: Option<PathBuf>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub duration: Option<i64>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub lyricists: Vec<String>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub composers: Vec<String>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub genres: Vec<String>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub labels: Vec<String>,
}

impl From<index::Song> for Song {
	fn from(s: index::Song) -> Self {
		Self {
			path: s.virtual_path,
			track_number: s.track_number,
			disc_number: s.disc_number,
			title: s.title,
			artists: s.artists,
			album_artists: s.album_artists,
			year: s.year,
			album: s.album,
			artwork: s.artwork,
			duration: s.duration,
			lyricists: s.lyricists,
			composers: s.composers,
			genres: s.genres,
			labels: s.labels,
		}
	}
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SongList {
	pub paths: Vec<PathBuf>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserEntry {
	pub path: PathBuf,
	pub is_directory: bool,
}

impl From<index::File> for BrowserEntry {
	fn from(file: index::File) -> Self {
		match file {
			index::File::Directory(d) => Self {
				is_directory: true,
				path: d,
			},
			index::File::Song(s) => Self {
				is_directory: false,
				path: s,
			},
		}
	}
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtistHeader {
	pub name: Option<String>,
	pub num_albums: u32,
}

impl From<index::ArtistHeader> for ArtistHeader {
	fn from(a: index::ArtistHeader) -> Self {
		Self {
			name: a.name.map(|s| s.into_inner()),
			num_albums: a.num_albums,
		}
	}
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artist {
	#[serde(flatten)]
	pub header: ArtistHeader,
	pub albums: Vec<AlbumHeader>,
	pub featured_on: Vec<AlbumHeader>,
}

impl From<index::Artist> for Artist {
	fn from(a: index::Artist) -> Self {
		Self {
			header: a.header.into(),
			albums: a.albums.into_iter().map(|a| a.into()).collect(),
			featured_on: a.featured_on.into_iter().map(|a| a.into()).collect(),
		}
	}
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlbumHeader {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub artwork: Option<String>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub artists: Vec<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub year: Option<i64>,
}

impl From<index::Album> for AlbumHeader {
	fn from(a: index::Album) -> Self {
		Self {
			name: a.name,
			artwork: a.artwork.map(|a| a.to_string_lossy().to_string()),
			artists: a.artists,
			year: a.year,
		}
	}
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Album {
	#[serde(flatten)]
	pub header: AlbumHeader,
	pub songs: Vec<Song>,
}

impl From<index::Album> for Album {
	fn from(mut a: index::Album) -> Self {
		let songs = a.songs.drain(..).map(|s| s.into()).collect();
		Self {
			header: a.into(),
			songs: songs,
		}
	}
}

// TODO: Preferences should have dto types
