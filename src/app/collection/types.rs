use std::path::PathBuf;

use sqlx::prelude::FromRow;

use crate::{
	app::vfs::{self},
	db,
};

#[derive(Clone, Debug, FromRow, PartialEq, Eq)]
pub struct MultiString(pub Vec<String>);

impl MultiString {
	pub const SEPARATOR: &'static str = "\u{000C}";
}

impl From<Option<String>> for MultiString {
	fn from(value: Option<String>) -> Self {
		match value {
			None => Self(Vec::new()),
			Some(s) => Self(s.split(Self::SEPARATOR).map(|s| s.to_string()).collect()),
		}
	}
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("Directory not found: {0}")]
	DirectoryNotFound(PathBuf),
	#[error(transparent)]
	Database(#[from] sqlx::Error),
	#[error(transparent)]
	DatabaseConnection(#[from] db::Error),
	#[error(transparent)]
	Vfs(#[from] vfs::Error),
	#[error(transparent)]
	ThreadPoolBuilder(#[from] rayon::ThreadPoolBuildError),
	#[error(transparent)]
	ThreadJoining(#[from] tokio::task::JoinError),
}

#[derive(Debug, PartialEq, Eq)]
pub enum File {
	Directory(Directory),
	Song(Song),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Song {
	pub id: i64,
	pub path: String,
	pub virtual_path: String,
	pub virtual_parent: String,
	pub track_number: Option<i64>,
	pub disc_number: Option<i64>,
	pub title: Option<String>,
	pub artists: MultiString,
	pub album_artists: MultiString,
	pub year: Option<i64>,
	pub album: Option<String>,
	pub artwork: Option<String>,
	pub duration: Option<i64>,
	pub lyricists: MultiString,
	pub composers: MultiString,
	pub genres: MultiString,
	pub labels: MultiString,
	pub date_added: i64,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Directory {
	pub id: i64,
	pub path: String,
	pub virtual_path: String,
	pub virtual_parent: Option<String>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Album {
	pub name: Option<String>,
	pub artwork: Option<String>,
	pub artists: Vec<String>,
	pub year: Option<i64>,
	pub date_added: i64,
}
