use std::{borrow::Cow, path::Path};

use sqlx::{
	encode::IsNull,
	sqlite::{SqliteArgumentValue, SqliteTypeInfo},
	Sqlite,
};

use crate::{
	app::vfs::{self, VFS},
	db,
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error(transparent)]
	IndexClean(#[from] super::cleaner::Error),
	#[error(transparent)]
	Database(#[from] sqlx::Error),
	#[error(transparent)]
	DatabaseConnection(#[from] db::Error),
	#[error(transparent)]
	Vfs(#[from] vfs::Error),
}

#[derive(Debug, PartialEq, Eq)]
pub struct MultiString(pub Vec<String>);

static MULTI_STRING_SEPARATOR: &str = "\u{000C}";

impl<'q> sqlx::Encode<'q, Sqlite> for MultiString {
	fn encode_by_ref(&self, args: &mut Vec<SqliteArgumentValue<'q>>) -> IsNull {
		if self.0.is_empty() {
			IsNull::Yes
		} else {
			let joined = self.0.join(MULTI_STRING_SEPARATOR);
			args.push(SqliteArgumentValue::Text(Cow::Owned(joined)));
			IsNull::No
		}
	}
}

impl From<Option<String>> for MultiString {
	fn from(value: Option<String>) -> Self {
		match value {
			None => MultiString(Vec::new()),
			Some(s) => MultiString(
				s.split(MULTI_STRING_SEPARATOR)
					.map(|s| s.to_string())
					.collect(),
			),
		}
	}
}

impl sqlx::Type<Sqlite> for MultiString {
	fn type_info() -> SqliteTypeInfo {
		<&str as sqlx::Type<Sqlite>>::type_info()
	}
}

#[derive(Debug, PartialEq, Eq)]
pub struct Song {
	pub id: i64,
	pub path: String,
	pub parent: String,
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
}

impl Song {
	pub fn virtualize(mut self, vfs: &VFS) -> Option<Song> {
		self.path = match vfs.real_to_virtual(Path::new(&self.path)) {
			Ok(p) => p.to_string_lossy().into_owned(),
			_ => return None,
		};
		if let Some(artwork_path) = self.artwork {
			self.artwork = match vfs.real_to_virtual(Path::new(&artwork_path)) {
				Ok(p) => Some(p.to_string_lossy().into_owned()),
				_ => None,
			};
		}
		Some(self)
	}
}

#[derive(Debug, PartialEq, Eq)]
pub struct Directory {
	pub id: i64,
	pub path: String,
	pub parent: Option<String>,
	// TODO remove all below when explorer and metadata browsing are separate
	pub artists: MultiString,
	pub year: Option<i64>,
	pub album: Option<String>,
	pub artwork: Option<String>,
	pub date_added: i64,
}

impl Directory {
	pub fn virtualize(mut self, vfs: &VFS) -> Option<Directory> {
		self.path = match vfs.real_to_virtual(Path::new(&self.path)) {
			Ok(p) => p.to_string_lossy().into_owned(),
			_ => return None,
		};
		if let Some(artwork_path) = self.artwork {
			self.artwork = match vfs.real_to_virtual(Path::new(&artwork_path)) {
				Ok(p) => Some(p.to_string_lossy().into_owned()),
				_ => None,
			};
		}
		Some(self)
	}
}
