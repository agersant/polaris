use std::{
	collections::HashMap,
	ops::Deref,
	path::{Path, PathBuf},
	str::FromStr,
};

use regex::Regex;
use rusqlite::Connection;

use crate::app::{config, index, scanner, Error};

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

	let album_art_pattern: String = connection.query_row(
		"SELECT index_album_art_pattern FROM misc_settings",
		[],
		|row| row.get(0),
	)?;

	let mount_dirs = read_mount_dirs(db_file_path)?;
	let users = read_users(db_file_path)?;

	Ok(Some(config::storage::Config {
		album_art_pattern: Some(album_art_pattern),
		mount_dirs,
		ddns_update_url: None,
		users: users.into_values().collect(),
	}))
}

fn read_mount_dirs(db_file_path: &PathBuf) -> Result<Vec<config::storage::MountDir>, Error> {
	let connection = Connection::open(db_file_path)?;

	let mut mount_dirs_statement = connection.prepare("SELECT source, name FROM mount_points")?;
	let mount_dirs_rows = mount_dirs_statement.query_and_then::<_, Error, _, _>([], |row| {
		let source_string = row.get::<_, String>(0)?;
		let source = PathBuf::from_str(&source_string)
			.map_err(|_| Error::InvalidDirectory(source_string))?;
		Ok(config::storage::MountDir {
			source,
			name: row.get::<_, String>(1)?,
		})
	})?;

	let mut mount_dirs = vec![];
	for mount_dir_result in mount_dirs_rows {
		mount_dirs.push(mount_dir_result?);
	}

	Ok(mount_dirs)
}

fn read_users(db_file_path: &PathBuf) -> Result<HashMap<u32, config::storage::User>, Error> {
	let connection = Connection::open(db_file_path)?;
	let mut users_statement =
		connection.prepare("SELECT id, name, password_hash, admin FROM users")?;
	let users_rows = users_statement.query_map([], |row| {
		Ok((
			row.get(0)?,
			config::storage::User {
				name: row.get(1)?,
				admin: row.get(3)?,
				initial_password: None,
				hashed_password: row.get(2)?,
			},
		))
	})?;

	let mut users = HashMap::new();
	for users_row in users_rows {
		let (id, user) = users_row?;
		users.insert(id, user);
	}

	Ok(users)
}

fn sanitize_path(source: &Path) -> PathBuf {
	let path_string = source.to_string_lossy();
	let separator_regex = Regex::new(r"\\|/").unwrap();
	let mut correct_separator = String::new();
	correct_separator.push(std::path::MAIN_SEPARATOR);
	let path_string = separator_regex.replace_all(&path_string, correct_separator.as_str());
	PathBuf::from(path_string.deref())
}

fn virtualize_path(
	real_path: &Path,
	mount_dirs: &Vec<config::storage::MountDir>,
) -> Result<PathBuf, Error> {
	let sanitized = sanitize_path(real_path); // Paths in test database use `/` separators, but need `\` when running tests on Windows
	for mount_dir in mount_dirs {
		if let Ok(tail) = sanitized.strip_prefix(&mount_dir.source) {
			return Ok(Path::new(&mount_dir.name).join(tail));
		}
	}
	Err(Error::CouldNotMapToVirtualPath(real_path.to_path_buf()))
}

pub async fn read_legacy_playlists(
	db_file_path: &PathBuf,
	index_manager: index::Manager,
	scanner: scanner::Scanner,
) -> Result<Vec<(String, String, Vec<index::Song>)>, Error> {
	scanner.run_scan().await?;

	let users = read_users(db_file_path)?;
	let mount_dirs = read_mount_dirs(db_file_path)?;
	let connection = Connection::open(db_file_path)?;

	let mut playlists_statement = connection.prepare("SELECT id, owner, name FROM playlists")?;
	let playlists_rows =
		playlists_statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
	let mut playlists = HashMap::new();
	for playlists_row in playlists_rows {
		let (id, owner, name): (u32, u32, String) = playlists_row?;
		playlists.insert(id, (users.get(&owner).ok_or(Error::UserNotFound)?, name));
	}

	let mut playlists_by_user: HashMap<String, HashMap<String, Vec<index::Song>>> = HashMap::new();
	let mut songs_statement =
		connection.prepare("SELECT playlist, path FROM playlist_songs ORDER BY ordering")?;
	let mut songs_rows = songs_statement.query([])?;
	while let Some(row) = songs_rows.next()? {
		let playlist = playlists.get(&row.get(0)?).ok_or(Error::PlaylistNotFound)?;
		let user = playlist.0.name.clone();
		let name = playlist.1.clone();
		let real_path = PathBuf::from(row.get::<_, String>(1)?);
		let Ok(virtual_path) = virtualize_path(&real_path, &mount_dirs) else {
			continue;
		};
		let Ok(song) = index_manager
			.get_songs(vec![virtual_path])
			.await
			.pop()
			.unwrap()
		else {
			continue;
		};
		playlists_by_user
			.entry(user)
			.or_default()
			.entry(name)
			.or_default()
			.push(song);
	}

	let mut results = vec![];
	for (user, playlists) in playlists_by_user {
		for (playlist_name, songs) in playlists {
			results.push((playlist_name.clone(), user.clone(), songs));
		}
	}

	Ok(results)
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
	use crate::{
		app::{config, test},
		test_name,
	};

	#[test]
	fn can_read_auth_secret() {
		let secret =
			read_legacy_auth_secret(&PathBuf::from_iter(["test-data", "legacy_db_blank.sqlite"]))
				.unwrap();
		assert_eq!(
			secret,
			[
				0x8B as u8, 0x88, 0x50, 0x17, 0x20, 0x09, 0x7E, 0x60, 0x31, 0x80, 0xCE, 0xE3, 0xF0,
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
				source: PathBuf::from_iter(["test-data", "small-collection"]),
				name: "root".to_owned(),
			}],
			ddns_update_url: None,
			users: vec![config::storage::User {
				name: "example_user".to_owned(),
				admin: Some(true),
				initial_password: None,
				hashed_password: Some("$pbkdf2-sha256$i=10000,l=32$ADvDnwBv3kLUtjTJEwGcFA$oK43ICpNt2rbH21diMo6cSXL62qqLWOM7qs8f0s/9Oo".to_owned()),
			}],
		};

		assert_eq!(actual, expected);
	}

	#[tokio::test]
	async fn can_read_blank_playlists() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;

		let actual = read_legacy_playlists(
			&PathBuf::from_iter(["test-data", "legacy_db_blank.sqlite"]),
			ctx.index_manager,
			ctx.scanner,
		)
		.await
		.unwrap();

		let expected = vec![];

		assert_eq!(actual, expected);
	}

	#[tokio::test]
	async fn can_read_populated_playlists() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;
		let db_file_path = PathBuf::from_iter(["test-data", "legacy_db_populated.sqlite"]);

		let config = read_legacy_config(&db_file_path).unwrap().unwrap();
		ctx.config_manager.apply_config(config).await.unwrap();

		let actual = read_legacy_playlists(
			&db_file_path,
			ctx.index_manager.clone(),
			ctx.scanner.clone(),
		)
		.await
		.unwrap();

		#[rustfmt::skip]
		let song_paths = vec![
			PathBuf::from_iter(["root", "Khemmis", "Hunted", "01 - Above The Water.mp3"]),
			PathBuf::from_iter(["root", "Khemmis", "Hunted", "02 - Candlelight.mp3"]),
			PathBuf::from_iter(["root", "Khemmis", "Hunted", "03 - Three Gates.mp3"]),
			PathBuf::from_iter(["root", "Khemmis", "Hunted", "04 - Beyond The Door.mp3"]),
			PathBuf::from_iter(["root", "Khemmis", "Hunted", "05 - Hunted.mp3"]),
		];

		let songs: Vec<index::Song> = ctx
			.index_manager
			.get_songs(song_paths)
			.await
			.into_iter()
			.map(|s| s.unwrap())
			.collect();

		let expected = vec![(
			"Example Playlist".to_owned(),
			"example_user".to_owned(),
			songs,
		)];

		assert_eq!(actual, expected);
	}
}
