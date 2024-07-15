use std::path::PathBuf;

use crate::{
	app::{scanner, vfs},
	db,
};

#[derive(Debug, PartialEq, Eq)]
pub enum CollectionFile {
	Directory(scanner::Directory),
	Song(scanner::Song),
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error(transparent)]
	Database(#[from] sqlx::Error),
	#[error(transparent)]
	DatabaseConnection(#[from] db::Error),
	#[error("Song was not found: `{0}`")]
	SongNotFound(PathBuf),
	#[error(transparent)]
	Vfs(#[from] vfs::Error),
}
