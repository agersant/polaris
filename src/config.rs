use core::ops::Deref;
use diesel;
use diesel::prelude::*;
use error_chain::bail;
use log::info;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path;
use toml;

use crate::db::ConnectionSource;
use crate::db::DB;
use crate::db::{ddns_config, misc_settings, mount_points, users};
use crate::ddns::DDNSConfig;
use crate::errors::*;
use crate::user::*;
use crate::vfs::MountPoint;

#[derive(Debug, Queryable)]
pub struct MiscSettings {
	id: i32,
	pub auth_secret: Vec<u8>,
	pub index_sleep_duration_seconds: i32,
	pub index_album_art_pattern: String,
	pub prefix_url: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Preferences {
	pub lastfm_username: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConfigUser {
	pub name: String,
	pub password: String,
	pub admin: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
	pub album_art_pattern: Option<String>,
	pub reindex_every_n_seconds: Option<i32>,
	pub mount_dirs: Option<Vec<MountPoint>>,
	pub prefix_url: Option<String>,
	pub users: Option<Vec<ConfigUser>>,
	pub ydns: Option<DDNSConfig>,
}

impl Config {
	fn clean_paths(&mut self) -> Result<()> {
		if let Some(ref mut mount_dirs) = self.mount_dirs {
			for mount_dir in mount_dirs {
				match clean_path_string(&mount_dir.source).to_str() {
					Some(p) => mount_dir.source = p.to_owned(),
					_ => bail!("Bad mount directory path"),
				}
			}
		}
		Ok(())
	}
}

pub fn parse_toml_file(path: &path::Path) -> Result<Config> {
	info!("Config file path: {}", path.to_string_lossy());
	let mut config_file = fs::File::open(path)?;
	let mut config_file_content = String::new();
	config_file.read_to_string(&mut config_file_content)?;
	let mut config = toml::de::from_str::<Config>(&config_file_content)?;
	config.clean_paths()?;
	Ok(config)
}

pub fn read<T>(db: &T) -> Result<Config>
where
	T: ConnectionSource,
{
	use self::ddns_config::dsl::*;
	use self::misc_settings::dsl::*;

	let connection = db.get_connection();

	let mut config = Config {
		album_art_pattern: None,
		reindex_every_n_seconds: None,
		mount_dirs: None,
		prefix_url: None,
		users: None,
		ydns: None,
	};

	let (art_pattern, sleep_duration, url) = misc_settings
		.select((
			index_album_art_pattern,
			index_sleep_duration_seconds,
			prefix_url,
		))
		.get_result(connection.deref())?;

	config.album_art_pattern = Some(art_pattern);
	config.reindex_every_n_seconds = Some(sleep_duration);
	config.prefix_url = if url != "" { Some(url) } else { None };

	let mount_dirs;
	{
		use self::mount_points::dsl::*;
		mount_dirs = mount_points
			.select((source, name))
			.get_results(connection.deref())?;
		config.mount_dirs = Some(mount_dirs);
	}

	let found_users: Vec<(String, i32)> = users::table
		.select((users::columns::name, users::columns::admin))
		.get_results(connection.deref())?;
	config.users = Some(
		found_users
			.into_iter()
			.map(|(name, admin)| ConfigUser {
				name,
				password: "".to_owned(),
				admin: admin != 0,
			})
			.collect::<_>(),
	);

	let ydns = ddns_config
		.select((host, username, password))
		.get_result(connection.deref())?;
	config.ydns = Some(ydns);

	Ok(config)
}

#[cfg(test)]
pub fn reset<T>(db: &T) -> Result<()>
where
	T: ConnectionSource,
{
	use self::ddns_config::dsl::*;
	let connection = db.get_connection();

	diesel::delete(mount_points::table).execute(connection.deref())?;
	diesel::delete(users::table).execute(connection.deref())?;
	diesel::update(ddns_config)
		.set((host.eq(""), username.eq(""), password.eq("")))
		.execute(connection.deref())?;

	Ok(())
}

pub fn amend<T>(db: &T, new_config: &Config) -> Result<()>
where
	T: ConnectionSource,
{
	let connection = db.get_connection();

	if let Some(ref mount_dirs) = new_config.mount_dirs {
		diesel::delete(mount_points::table).execute(connection.deref())?;
		diesel::insert_into(mount_points::table)
			.values(mount_dirs)
			.execute(connection.deref())?;
	}

	if let Some(ref config_users) = new_config.users {
		let old_usernames: Vec<String> = users::table
			.select(users::name)
			.get_results(connection.deref())?;

		// Delete users that are not in new list
		let delete_usernames: Vec<String> = old_usernames
			.iter()
			.cloned()
			.filter(|old_name| config_users.iter().find(|u| &u.name == old_name).is_none())
			.collect::<_>();
		diesel::delete(users::table.filter(users::name.eq_any(&delete_usernames)))
			.execute(connection.deref())?;

		// Insert new users
		let insert_users: Vec<&ConfigUser> = config_users
			.iter()
			.filter(|u| {
				!u.name.is_empty()
					&& !u.password.is_empty()
					&& old_usernames
						.iter()
						.find(|old_name| *old_name == &u.name)
						.is_none()
			})
			.collect::<_>();
		for config_user in &insert_users {
			let new_user = User::new(&config_user.name, &config_user.password)?;
			diesel::insert_into(users::table)
				.values(&new_user)
				.execute(connection.deref())?;
		}

		// Update users
		for user in config_users.iter() {
			// Update password if provided
			if !user.password.is_empty() {
				let hash = hash_password(&user.password)?;
				diesel::update(users::table.filter(users::name.eq(&user.name)))
					.set(users::password_hash.eq(hash))
					.execute(connection.deref())?;
			}

			// Update admin rights
			diesel::update(users::table.filter(users::name.eq(&user.name)))
				.set(users::admin.eq(user.admin as i32))
				.execute(connection.deref())?;
		}
	}

	if let Some(sleep_duration) = new_config.reindex_every_n_seconds {
		diesel::update(misc_settings::table)
			.set(misc_settings::index_sleep_duration_seconds.eq(sleep_duration as i32))
			.execute(connection.deref())?;
	}

	if let Some(ref album_art_pattern) = new_config.album_art_pattern {
		diesel::update(misc_settings::table)
			.set(misc_settings::index_album_art_pattern.eq(album_art_pattern))
			.execute(connection.deref())?;
	}

	if let Some(ref ydns) = new_config.ydns {
		use self::ddns_config::dsl::*;
		diesel::update(ddns_config)
			.set((
				host.eq(ydns.host.clone()),
				username.eq(ydns.username.clone()),
				password.eq(ydns.password.clone()),
			))
			.execute(connection.deref())?;
	}

	if let Some(ref prefix_url) = new_config.prefix_url {
		diesel::update(misc_settings::table)
			.set(misc_settings::prefix_url.eq(prefix_url))
			.execute(connection.deref())?;
	}

	Ok(())
}

pub fn read_preferences<T>(db: &T, username: &str) -> Result<Preferences>
where
	T: ConnectionSource,
{
	use self::users::dsl::*;
	let connection = db.get_connection();
	let read_lastfm_username = users
		.select(lastfm_username)
		.filter(name.eq(username))
		.get_result(connection.deref())?;
	Ok(Preferences {
		lastfm_username: read_lastfm_username,
	})
}

pub fn write_preferences<T>(_: &T, _: &str, _: &Preferences) -> Result<()>
where
	T: ConnectionSource,
{
	Ok(())
}

pub fn get_auth_secret<T>(db: &T) -> Result<Vec<u8>>
where
	T: ConnectionSource,
{
	use self::misc_settings::dsl::*;

	let connection = db.get_connection();

	match misc_settings
		.select(auth_secret)
		.get_result(connection.deref())
	{
		Err(diesel::result::Error::NotFound) => bail!("Cannot find authentication secret"),
		Ok(secret) => Ok(secret),
		Err(e) => Err(e.into()),
	}
}

fn clean_path_string(path_string: &str) -> path::PathBuf {
	let separator_regex = Regex::new(r"\\|/").unwrap();
	let mut correct_separator = String::new();
	correct_separator.push(path::MAIN_SEPARATOR);
	let path_string = separator_regex.replace_all(path_string, correct_separator.as_str());
	path::Path::new(path_string.deref()).iter().collect()
}

fn _get_test_db(name: &str) -> DB {
	let mut db_path = path::PathBuf::new();
	db_path.push("test");
	db_path.push(name);
	if db_path.exists() {
		fs::remove_file(&db_path).unwrap();
	}

	DB::new(&db_path).unwrap()
}

#[test]
fn test_amend() {
	let db = _get_test_db("amend.sqlite");

	let initial_config = Config {
		album_art_pattern: Some("file\\.png".into()),
		reindex_every_n_seconds: Some(123),
		prefix_url: None,
		mount_dirs: Some(vec![MountPoint {
			source: "C:\\Music".into(),
			name: "root".into(),
		}]),
		users: Some(vec![ConfigUser {
			name: "Teddy🐻".into(),
			password: "Tasty🍖".into(),
			admin: false,
		}]),
		ydns: None,
	};

	let new_config = Config {
		album_art_pattern: Some("🖼️\\.jpg".into()),
		reindex_every_n_seconds: None,
		prefix_url: Some("polaris".into()),
		mount_dirs: Some(vec![MountPoint {
			source: "/home/music".into(),
			name: "🎵📁".into(),
		}]),
		users: Some(vec![ConfigUser {
			name: "Kermit🐸".into(),
			password: "🐞🐞".into(),
			admin: false,
		}]),
		ydns: Some(DDNSConfig {
			host: "🐸🐸🐸.ydns.eu".into(),
			username: "kfr🐸g".into(),
			password: "tasty🐞".into(),
		}),
	};

	let mut expected_config = new_config.clone();
	expected_config.reindex_every_n_seconds = initial_config.reindex_every_n_seconds;
	if let Some(ref mut users) = expected_config.users {
		users[0].password = "".into();
	}

	amend(&db, &initial_config).unwrap();
	amend(&db, &new_config).unwrap();
	let db_config = read(&db).unwrap();
	assert_eq!(db_config, expected_config);
}

#[test]
fn test_amend_preserve_password_hashes() {
	use self::users::dsl::*;

	let db = _get_test_db("amend_preserve_password_hashes.sqlite");
	let initial_hash: String;
	let new_hash: String;

	let initial_config = Config {
		album_art_pattern: None,
		reindex_every_n_seconds: None,
		prefix_url: None,
		mount_dirs: None,
		users: Some(vec![ConfigUser {
			name: "Teddy🐻".into(),
			password: "Tasty🍖".into(),
			admin: false,
		}]),
		ydns: None,
	};
	amend(&db, &initial_config).unwrap();

	{
		let connection = db.get_connection();
		initial_hash = users
			.select(password_hash)
			.filter(name.eq("Teddy🐻"))
			.get_result(connection.deref())
			.unwrap();
	}

	let new_config = Config {
		album_art_pattern: None,
		reindex_every_n_seconds: None,
		prefix_url: None,
		mount_dirs: None,
		users: Some(vec![
			ConfigUser {
				name: "Kermit🐸".into(),
				password: "tasty🐞".into(),
				admin: false,
			},
			ConfigUser {
				name: "Teddy🐻".into(),
				password: "".into(),
				admin: false,
			},
		]),
		ydns: None,
	};
	amend(&db, &new_config).unwrap();

	{
		let connection = db.get_connection();
		new_hash = users
			.select(password_hash)
			.filter(name.eq("Teddy🐻"))
			.get_result(connection.deref())
			.unwrap();
	}

	assert_eq!(new_hash, initial_hash);
}

#[test]
fn test_amend_ignore_blank_users() {
	use self::users::dsl::*;

	let db = _get_test_db("amend_ignore_blank_users.sqlite");

	{
		let config = Config {
			album_art_pattern: None,
			reindex_every_n_seconds: None,
			prefix_url: None,
			mount_dirs: None,
			users: Some(vec![ConfigUser {
				name: "".into(),
				password: "Tasty🍖".into(),
				admin: false,
			}]),
			ydns: None,
		};
		amend(&db, &config).unwrap();

		let connection = db.get_connection();
		let user_count: i64 = users.count().get_result(connection.deref()).unwrap();
		assert_eq!(user_count, 0);
	}

	{
		let config = Config {
			album_art_pattern: None,
			reindex_every_n_seconds: None,
			prefix_url: None,
			mount_dirs: None,
			users: Some(vec![ConfigUser {
				name: "Teddy🐻".into(),
				password: "".into(),
				admin: false,
			}]),
			ydns: None,
		};
		amend(&db, &config).unwrap();

		let connection = db.get_connection();
		let user_count: i64 = users.count().get_result(connection.deref()).unwrap();
		assert_eq!(user_count, 0);
	}
}

#[test]
fn test_toggle_admin() {
	use self::users::dsl::*;

	let db = _get_test_db("amend_toggle_admin.sqlite");

	let initial_config = Config {
		album_art_pattern: None,
		reindex_every_n_seconds: None,
		prefix_url: None,
		mount_dirs: None,
		users: Some(vec![ConfigUser {
			name: "Teddy🐻".into(),
			password: "Tasty🍖".into(),
			admin: true,
		}]),
		ydns: None,
	};
	amend(&db, &initial_config).unwrap();

	{
		let connection = db.get_connection();
		let is_admin: i32 = users.select(admin).get_result(connection.deref()).unwrap();
		assert_eq!(is_admin, 1);
	}

	let new_config = Config {
		album_art_pattern: None,
		reindex_every_n_seconds: None,
		prefix_url: None,
		mount_dirs: None,
		users: Some(vec![ConfigUser {
			name: "Teddy🐻".into(),
			password: "".into(),
			admin: false,
		}]),
		ydns: None,
	};
	amend(&db, &new_config).unwrap();

	{
		let connection = db.get_connection();
		let is_admin: i32 = users.select(admin).get_result(connection.deref()).unwrap();
		assert_eq!(is_admin, 0);
	}
}

#[test]
fn test_preferences_read_write() {
	let db = _get_test_db("preferences_read_write.sqlite");

	let initial_config = Config {
		album_art_pattern: None,
		reindex_every_n_seconds: None,
		prefix_url: None,
		mount_dirs: None,
		users: Some(vec![ConfigUser {
			name: "Teddy🐻".into(),
			password: "Tasty🍖".into(),
			admin: false,
		}]),
		ydns: None,
	};
	amend(&db, &initial_config).unwrap();

	let new_preferences = Preferences {
		lastfm_username: None,
	};
	write_preferences(&db, "Teddy🐻", &new_preferences).unwrap();

	let read_preferences = read_preferences(&db, "Teddy🐻").unwrap();
	assert_eq!(new_preferences, read_preferences);
}

#[test]
fn test_clean_path_string() {
	let mut correct_path = path::PathBuf::new();
	if cfg!(target_os = "windows") {
		correct_path.push("C:\\");
	} else {
		correct_path.push("/usr");
	}
	correct_path.push("some");
	correct_path.push("path");
	if cfg!(target_os = "windows") {
		assert_eq!(correct_path, clean_path_string(r#"C:/some/path"#));
		assert_eq!(correct_path, clean_path_string(r#"C:\some\path"#));
		assert_eq!(correct_path, clean_path_string(r#"C:\some\path\"#));
		assert_eq!(correct_path, clean_path_string(r#"C:\some\path\\\\"#));
		assert_eq!(correct_path, clean_path_string(r#"C:\some/path//"#));
	} else {
		assert_eq!(correct_path, clean_path_string(r#"/usr/some/path"#));
		assert_eq!(correct_path, clean_path_string(r#"/usr\some\path"#));
		assert_eq!(correct_path, clean_path_string(r#"/usr\some\path\"#));
		assert_eq!(correct_path, clean_path_string(r#"/usr\some\path\\\\"#));
		assert_eq!(correct_path, clean_path_string(r#"/usr\some/path//"#));
	}
}
