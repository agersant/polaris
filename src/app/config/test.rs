use diesel::prelude::*;
use std::fs;
use std::path::PathBuf;

use super::*;
use crate::app::{user, vfs};
use crate::db::{users, DB};
use crate::test_name;

#[cfg(test)]
fn get_test_db(name: &str) -> DB {
	let mut db_path = PathBuf::new();
	db_path.push("test-output");
	fs::create_dir_all(&db_path).unwrap();

	db_path.push(name);
	if db_path.exists() {
		fs::remove_file(&db_path).unwrap();
	}

	DB::new(&db_path).unwrap()
}

#[test]
fn test_amend() {
	let db = get_test_db(&test_name!());
	let user_manager = user::Manager::new(db.clone());
	let config_manager = Manager::new(db, user_manager);

	let initial_config = Config {
		album_art_pattern: Some("file\\.png".into()),
		reindex_every_n_seconds: Some(123),
		mount_dirs: Some(vec![vfs::MountPoint {
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
		mount_dirs: Some(vec![vfs::MountPoint {
			source: "/home/music".into(),
			name: "🎵📁".into(),
		}]),
		users: Some(vec![ConfigUser {
			name: "Kermit🐸".into(),
			password: "🐞🐞".into(),
			admin: false,
		}]),
		ydns: Some(ddns::Config {
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

	config_manager.amend(&initial_config).unwrap();
	config_manager.amend(&new_config).unwrap();
	let db_config = config_manager.read().unwrap();
	assert_eq!(db_config, expected_config);
}

#[test]
fn test_amend_preserve_password_hashes() {
	use self::users::dsl::*;

	let db = get_test_db(&test_name!());
	let user_manager = user::Manager::new(db.clone());
	let config_manager = Manager::new(db.clone(), user_manager);

	let initial_hash: String;
	let new_hash: String;

	let initial_config = Config {
		album_art_pattern: None,
		reindex_every_n_seconds: None,
		mount_dirs: None,
		users: Some(vec![ConfigUser {
			name: "Teddy🐻".into(),
			password: "Tasty🍖".into(),
			admin: false,
		}]),
		ydns: None,
	};
	config_manager.amend(&initial_config).unwrap();

	{
		let connection = db.connect().unwrap();
		initial_hash = users
			.select(password_hash)
			.filter(name.eq("Teddy🐻"))
			.get_result(&connection)
			.unwrap();
	}

	let new_config = Config {
		album_art_pattern: None,
		reindex_every_n_seconds: None,
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
	config_manager.amend(&new_config).unwrap();

	{
		let connection = db.connect().unwrap();
		new_hash = users
			.select(password_hash)
			.filter(name.eq("Teddy🐻"))
			.get_result(&connection)
			.unwrap();
	}

	assert_eq!(new_hash, initial_hash);
}

#[test]
fn test_amend_ignore_blank_users() {
	use self::users::dsl::*;

	let db = get_test_db(&test_name!());
	let user_manager = user::Manager::new(db.clone());
	let config_manager = Manager::new(db.clone(), user_manager);

	{
		let config = Config {
			album_art_pattern: None,
			reindex_every_n_seconds: None,
			mount_dirs: None,
			users: Some(vec![ConfigUser {
				name: "".into(),
				password: "Tasty🍖".into(),
				admin: false,
			}]),
			ydns: None,
		};
		config_manager.amend(&config).unwrap();

		let connection = db.connect().unwrap();
		let user_count: i64 = users.count().get_result(&connection).unwrap();
		assert_eq!(user_count, 0);
	}

	{
		let config = Config {
			album_art_pattern: None,
			reindex_every_n_seconds: None,
			mount_dirs: None,
			users: Some(vec![ConfigUser {
				name: "Teddy🐻".into(),
				password: "".into(),
				admin: false,
			}]),
			ydns: None,
		};
		config_manager.amend(&config).unwrap();

		let connection = db.connect().unwrap();
		let user_count: i64 = users.count().get_result(&connection).unwrap();
		assert_eq!(user_count, 0);
	}
}

#[test]
fn test_toggle_admin() {
	use self::users::dsl::*;

	let db = get_test_db(&test_name!());
	let user_manager = user::Manager::new(db.clone());
	let config_manager = Manager::new(db.clone(), user_manager);

	let initial_config = Config {
		album_art_pattern: None,
		reindex_every_n_seconds: None,
		mount_dirs: None,
		users: Some(vec![ConfigUser {
			name: "Teddy🐻".into(),
			password: "Tasty🍖".into(),
			admin: true,
		}]),
		ydns: None,
	};
	config_manager.amend(&initial_config).unwrap();

	{
		let connection = db.connect().unwrap();
		let is_admin: i32 = users.select(admin).get_result(&connection).unwrap();
		assert_eq!(is_admin, 1);
	}

	let new_config = Config {
		album_art_pattern: None,
		reindex_every_n_seconds: None,
		mount_dirs: None,
		users: Some(vec![ConfigUser {
			name: "Teddy🐻".into(),
			password: "".into(),
			admin: false,
		}]),
		ydns: None,
	};
	config_manager.amend(&new_config).unwrap();

	{
		let connection = db.connect().unwrap();
		let is_admin: i32 = users.select(admin).get_result(&connection).unwrap();
		assert_eq!(is_admin, 0);
	}
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
		assert_eq!(correct_path, Config::clean_path_string(r#"C:/some/path"#));
		assert_eq!(correct_path, Config::clean_path_string(r#"C:\some\path"#));
		assert_eq!(correct_path, Config::clean_path_string(r#"C:\some\path\"#));
		assert_eq!(
			correct_path,
			Config::clean_path_string(r#"C:\some\path\\\\"#)
		);
		assert_eq!(correct_path, Config::clean_path_string(r#"C:\some/path//"#));
	} else {
		assert_eq!(correct_path, Config::clean_path_string(r#"/usr/some/path"#));
		assert_eq!(correct_path, Config::clean_path_string(r#"/usr\some\path"#));
		assert_eq!(
			correct_path,
			Config::clean_path_string(r#"/usr\some\path\"#)
		);
		assert_eq!(
			correct_path,
			Config::clean_path_string(r#"/usr\some\path\\\\"#)
		);
		assert_eq!(
			correct_path,
			Config::clean_path_string(r#"/usr\some/path//"#)
		);
	}
}
