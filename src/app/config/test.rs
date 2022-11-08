use super::*;
use crate::app::{ddns, settings, test, user, vfs};
use crate::test_name;

#[test]
fn apply_saves_misc_settings() {
	let ctx = test::ContextBuilder::new(test_name!()).build();
	let new_config = Config {
		settings: Some(settings::NewSettings {
			album_art_pattern: Some("🖼️\\.jpg".into()),
			reindex_every_n_seconds: Some(100),
		}),
		..Default::default()
	};

	ctx.config_manager.apply(&new_config).unwrap();
	let settings = ctx.settings_manager.read().unwrap();
	let new_settings = new_config.settings.unwrap();
	assert_eq!(
		settings.index_album_art_pattern,
		new_settings.album_art_pattern.unwrap()
	);
	assert_eq!(
		settings.index_sleep_duration_seconds,
		new_settings.reindex_every_n_seconds.unwrap()
	);
}

#[test]
fn apply_saves_mount_points() {
	let ctx = test::ContextBuilder::new(test_name!()).build();

	let new_config = Config {
		mount_dirs: Some(vec![vfs::MountDir {
			source: "/home/music".into(),
			name: "🎵📁".into(),
		}]),
		..Default::default()
	};

	ctx.config_manager.apply(&new_config).unwrap();
	let actual_mount_dirs: Vec<vfs::MountDir> = ctx.vfs_manager.mount_dirs().unwrap();
	assert_eq!(actual_mount_dirs, new_config.mount_dirs.unwrap());
}

#[test]
fn apply_saves_ddns_settings() {
	let ctx = test::ContextBuilder::new(test_name!()).build();

	let new_config = Config {
		ydns: Some(ddns::Config {
			host: "🐸🐸🐸.ydns.eu".into(),
			username: "kfr🐸g".into(),
			password: "tasty🐞".into(),
		}),
		..Default::default()
	};

	ctx.config_manager.apply(&new_config).unwrap();
	let actual_ddns = ctx.ddns_manager.config().unwrap();
	assert_eq!(actual_ddns, new_config.ydns.unwrap());
}

#[test]
fn apply_can_toggle_admin() {
	let ctx = test::ContextBuilder::new(test_name!())
		.user("Walter", "Tasty🍖", true)
		.build();

	assert!(ctx.user_manager.list().unwrap()[0].is_admin());

	let new_config = Config {
		users: Some(vec![user::NewUser {
			name: "Walter".into(),
			password: "Tasty🍖".into(),
			admin: false,
		}]),
		..Default::default()
	};
	ctx.config_manager.apply(&new_config).unwrap();
	assert!(!ctx.user_manager.list().unwrap()[0].is_admin());
}
