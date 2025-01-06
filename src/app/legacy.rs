use std::path::PathBuf;

use crate::app::{config, index, Error};

pub fn read_legacy_config(
	db_file_path: &PathBuf,
) -> Result<Option<config::storage::Config>, Error> {
	Ok(None)
}

pub fn read_legacy_playlists(
	db_file_path: &PathBuf,
	config_manager: config::Manager,
	index_manager: index::Manager,
) -> Result<Vec<(String, String, Vec<index::Song>)>, Error> {
	Ok(vec![])
}

pub async fn delete_legacy_db(db_file_path: &PathBuf) -> Result<(), Error> {
	tokio::fs::remove_file(db_file_path)
		.await
		.map_err(|e| Error::Io(db_file_path.clone(), e))?;
	Ok(())
}
