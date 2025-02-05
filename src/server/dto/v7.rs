use serde::{Deserialize, Serialize};

use crate::app::{config, index, thumbnail};
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
pub struct User {
	pub name: String,
	pub is_admin: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewUser {
	pub name: String,
	pub password: String,
	pub admin: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserUpdate {
	pub new_password: Option<String>,
	pub new_is_admin: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct MountDir {
	pub source: PathBuf,
	pub name: String,
}

impl From<MountDir> for config::storage::MountDir {
	fn from(m: MountDir) -> Self {
		Self {
			name: m.name,
			source: m.source,
		}
	}
}

impl From<config::MountDir> for MountDir {
	fn from(m: config::MountDir) -> Self {
		Self {
			name: m.name,
			source: m.source,
		}
	}
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewSettings {
	pub album_art_pattern: Option<String>,
	pub reindex_every_n_seconds: Option<i64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
	pub album_art_pattern: String,
	pub reindex_every_n_seconds: i64,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CollectionFile {
	Directory(Directory),
	Song(Song),
}

impl From<index::File> for CollectionFile {
	fn from(f: index::File) -> Self {
		match f {
			index::File::Directory(d) => Self::Directory(Directory {
				path: d,
				artist: None,
				year: None,
				album: None,
				artwork: None,
			}),
			index::File::Song(s) => Self::Song(Song {
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

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
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

impl From<&PathBuf> for Song {
	fn from(path: &PathBuf) -> Self {
		Self {
			path: path.clone(),
			..Default::default()
		}
	}
}

impl From<index::Song> for Song {
	fn from(s: index::Song) -> Self {
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

impl From<index::Album> for Directory {
	fn from(album: index::Album) -> Self {
		let path = album
			.songs
			.first()
			.and_then(|s| s.virtual_path.parent())
			.map(PathBuf::from)
			.unwrap_or_default();

		Self {
			path,
			artist: match album.header.artists.is_empty() {
				true => None,
				false => Some(album.header.artists.join("")),
			},
			year: album.header.year,
			album: Some(album.header.name),
			artwork: album.header.artwork,
		}
	}
}
