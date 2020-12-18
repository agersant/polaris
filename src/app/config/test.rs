use std::fs;
use std::path::PathBuf;

use super::*;
use crate::app::{settings, user, vfs};
use crate::db::DB;
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
fn apply_saves_misc_settings() {
	let db = get_test_db(&test_name!());
	let settings_manager = settings::Manager::new(db.clone());
	let user_manager = user::Manager::new(db.clone());
	let vfs_manager = vfs::Manager::new(db.clone());
	let config_manager = Manager::new(
		settings_manager.clone(),
		user_manager.clone(),
		vfs_manager.clone(),
	);

	let new_config = Config {
		settings: Some(settings::NewSettings {
			album_art_pattern: Some("🖼️\\.jpg".into()),
			reindex_every_n_seconds: Some(100),
			..Default::default()
		}),
		..Default::default()
	};

	config_manager.apply(&new_config).unwrap();
	let settings = settings_manager.read().unwrap();
	let new_settings = new_config.settings.unwrap();
	assert_eq!(
		settings.album_art_pattern,
		new_settings.album_art_pattern.unwrap()
	);
	assert_eq!(
		settings.reindex_every_n_seconds,
		new_settings.reindex_every_n_seconds.unwrap()
	);
}

#[test]
fn apply_saves_mount_points() {
	let db = get_test_db(&test_name!());
	let settings_manager = settings::Manager::new(db.clone());
	let user_manager = user::Manager::new(db.clone());
	let vfs_manager = vfs::Manager::new(db.clone());
	let config_manager = Manager::new(
		settings_manager.clone(),
		user_manager.clone(),
		vfs_manager.clone(),
	);

	let new_config = Config {
		mount_dirs: Some(vec![vfs::MountDir {
			source: "/home/music".into(),
			name: "🎵📁".into(),
		}]),
		..Default::default()
	};

	config_manager.apply(&new_config).unwrap();
	let actual_mount_dirs: Vec<vfs::MountDir> = vfs_manager.mount_dirs().unwrap();
	assert_eq!(actual_mount_dirs, new_config.mount_dirs.unwrap());
}

#[test]
fn apply_saves_ddns_settings() {
	use crate::app::ddns;

	let db = get_test_db(&test_name!());
	let settings_manager = settings::Manager::new(db.clone());
	let user_manager = user::Manager::new(db.clone());
	let vfs_manager = vfs::Manager::new(db.clone());
	let config_manager = Manager::new(
		settings_manager.clone(),
		user_manager.clone(),
		vfs_manager.clone(),
	);

	let new_config = Config {
		settings: Some(settings::NewSettings {
			ydns: Some(ddns::Config {
				host: "🐸🐸🐸.ydns.eu".into(),
				username: "kfr🐸g".into(),
				password: "tasty🐞".into(),
			}),
			..Default::default()
		}),
		..Default::default()
	};

	config_manager.apply(&new_config).unwrap();
	let settings = settings_manager.read().unwrap();
	let new_settings = new_config.settings.unwrap();
	assert_eq!(settings.ydns, new_settings.ydns);
}

#[test]
fn apply_preserves_password_hashes() {
	let db = get_test_db(&test_name!());
	let settings_manager = settings::Manager::new(db.clone());
	let user_manager = user::Manager::new(db.clone());
	let vfs_manager = vfs::Manager::new(db.clone());
	let config_manager = Manager::new(
		settings_manager.clone(),
		user_manager.clone(),
		vfs_manager.clone(),
	);

	let initial_config = Config {
		users: Some(vec![user::NewUser {
			name: "Walter".into(),
			password: "Tasty🍖".into(),
			admin: false,
		}]),
		..Default::default()
	};
	config_manager.apply(&initial_config).unwrap();
	let initial_hash = &user_manager.list().unwrap()[0].password_hash;

	let new_config = Config {
		users: Some(vec![user::NewUser {
			name: "Walter".into(),
			password: "".into(),
			admin: false,
		}]),
		..Default::default()
	};
	config_manager.apply(&new_config).unwrap();
	let new_hash = &user_manager.list().unwrap()[0].password_hash;

	assert_eq!(new_hash, initial_hash);
}

#[test]
fn apply_can_toggle_admin() {
	let db = get_test_db(&test_name!());
	let settings_manager = settings::Manager::new(db.clone());
	let user_manager = user::Manager::new(db.clone());
	let vfs_manager = vfs::Manager::new(db.clone());
	let config_manager = Manager::new(
		settings_manager.clone(),
		user_manager.clone(),
		vfs_manager.clone(),
	);

	let initial_config = Config {
		users: Some(vec![user::NewUser {
			name: "Walter".into(),
			password: "Tasty🍖".into(),
			admin: true,
		}]),
		..Default::default()
	};
	config_manager.apply(&initial_config).unwrap();
	assert!(user_manager.list().unwrap()[0].is_admin());

	let new_config = Config {
		users: Some(vec![user::NewUser {
			name: "Walter".into(),
			password: "".into(),
			admin: false,
		}]),
		..Default::default()
	};
	config_manager.apply(&new_config).unwrap();
	assert!(!user_manager.list().unwrap()[0].is_admin());
}
