use std::{path::PathBuf, str::FromStr};

use rusqlite::Connection;

use crate::app::{config, index, Error};

pub fn read_legacy_auth_secret(db_file_path: &PathBuf) -> Result<[u8; 32], Error> {
	let connection = Connection::open(db_file_path)?;
	let auth_secret: [u8; 32] =
		connection.query_row("SELECT auth_secret FROM misc_settings", [], |row| {
			row.get(0)
		})?;
	Ok(auth_secret)
}

pub fn read_legacy_config(
	db_file_path: &PathBuf,
) -> Result<Option<config::storage::Config>, Error> {
	let connection = Connection::open(db_file_path)?;

	// Album art pattern
	let album_art_pattern: String = connection.query_row(
		"SELECT index_album_art_pattern FROM misc_settings",
		[],
		|row| row.get(0),
	)?;

	// Mount directories
	let mut mount_dirs_statement = connection.prepare("SELECT source, name FROM mount_points")?;
	let mount_dirs_rows = mount_dirs_statement.query_and_then([], |row| {
		let source_string = row.get::<_, String>(0)?;
		let Ok(source) = PathBuf::from_str(&source_string) else {
			return Err(Error::InvalidDirectory(source_string));
		};
		Ok(config::storage::MountDir {
			source,
			name: row.get::<_, String>(1)?,
		})
	})?;
	let mut mount_dirs = vec![];
	for mount_dir_result in mount_dirs_rows {
		mount_dirs.push(mount_dir_result?);
	}

	// Users
	let mut users_statement = connection.prepare("SELECT name, password_hash, admin FROM users")?;
	let users_rows = users_statement.query_map([], |row| {
		Ok(config::storage::User {
			name: row.get(0)?,
			admin: row.get(2)?,
			initial_password: None,
			hashed_password: row.get(1)?,
		})
	})?;
	let mut users = vec![];
	for user_result in users_rows {
		users.push(user_result?);
	}

	Ok(Some(config::storage::Config {
		album_art_pattern: Some(album_art_pattern),
		mount_dirs,
		ddns_update_url: None,
		users,
	}))
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

#[cfg(test)]
mod test {
	use std::path::PathBuf;

	use super::*;
	use crate::app::config;

	#[test]
	fn can_read_auth_secret() {
		let secret =
			read_legacy_auth_secret(&PathBuf::from_iter(["test-data", "legacy_db_blank.sqlite"]))
				.unwrap();
		assert_eq!(
			secret,
			[
				0x8b as u8, 0x88, 0x50, 0x17, 0x20, 0x09, 0x7e, 0x60, 0x31, 0x80, 0xCE, 0xE3, 0xF0,
				0x5A, 0x00, 0xBC, 0x3A, 0xF4, 0xDC, 0xFD, 0x2E, 0xB7, 0x5D, 0x33, 0x5D, 0x81, 0x2F,
				0x9A, 0xB4, 0x3A, 0x27, 0x2D
			]
		);
	}

	#[test]
	fn can_read_blank_config() {
		let actual =
			read_legacy_config(&PathBuf::from_iter(["test-data", "legacy_db_blank.sqlite"]))
				.unwrap()
				.unwrap();

		let expected = config::storage::Config {
			album_art_pattern: Some("Folder.(jpeg|jpg|png)".to_owned()),
			mount_dirs: vec![],
			ddns_update_url: None,
			users: vec![],
		};

		assert_eq!(actual, expected);
	}

	#[test]
	fn can_read_populated_config() {
		let actual = read_legacy_config(&PathBuf::from_iter([
			"test-data",
			"legacy_db_populated.sqlite",
		]))
		.unwrap()
		.unwrap();

		let expected = config::storage::Config {
			album_art_pattern: Some("Folder.(jpeg|jpg|png)".to_owned()),
			mount_dirs: vec![config::storage::MountDir {
				source: PathBuf::from_iter(["/", "home", "agersant", "music", "Electronic", "Bitpop"]),
				name: "root".to_owned(),
			}],
			ddns_update_url: None,
			users: vec![config::storage::User {
				name: "example_user".to_owned(),
				admin: Some(true),
				initial_password: None,
				hashed_password: Some("$pbkdf2-sha256$i=10000,l=32$feX5cP9SyQrZdBZsOQfO3Q$vqdraNc8ecco+CdFr+2Vp+PcIK6R75rs72YovNCwd7s".to_owned()),
			}],
		};

		assert_eq!(actual, expected);
	}
}
