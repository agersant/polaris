use serde::Deserialize;
use std::io::Read;
use std::path;

use crate::app::{ddns, settings, user, vfs};

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error(transparent)]
	Settings(#[from] settings::Error),
	#[error(transparent)]
	User(#[from] user::Error),
	#[error("Unspecified")]
	Unspecified,
}

impl From<anyhow::Error> for Error {
	fn from(_: anyhow::Error) -> Self {
		Error::Unspecified
	}
}

#[derive(Default, Deserialize)]
pub struct Config {
	pub settings: Option<settings::NewSettings>,
	pub mount_dirs: Option<Vec<vfs::MountDir>>,
	pub ydns: Option<ddns::Config>,
	pub users: Option<Vec<user::NewUser>>,
}

impl Config {
	pub fn from_path(path: &path::Path) -> anyhow::Result<Config> {
		let mut config_file = std::fs::File::open(path)?;
		let mut config_file_content = String::new();
		config_file.read_to_string(&mut config_file_content)?;
		let config = toml::de::from_str::<Self>(&config_file_content)?;
		Ok(config)
	}
}

#[derive(Clone)]
pub struct Manager {
	settings_manager: settings::Manager,
	user_manager: user::Manager,
	vfs_manager: vfs::Manager,
	ddns_manager: ddns::Manager,
}

impl Manager {
	pub fn new(
		settings_manager: settings::Manager,
		user_manager: user::Manager,
		vfs_manager: vfs::Manager,
		ddns_manager: ddns::Manager,
	) -> Self {
		Self {
			settings_manager,
			user_manager,
			vfs_manager,
			ddns_manager,
		}
	}

	pub fn apply(&self, config: &Config) -> Result<(), Error> {
		if let Some(new_settings) = &config.settings {
			self.settings_manager.amend(new_settings)?;
		}

		if let Some(mount_dirs) = &config.mount_dirs {
			self.vfs_manager.set_mount_dirs(mount_dirs)?;
		}

		if let Some(ddns_config) = &config.ydns {
			self.ddns_manager.set_config(ddns_config)?;
		}

		if let Some(ref users) = config.users {
			let old_users: Vec<user::User> = self.user_manager.list()?;

			// Delete users that are not in new list
			for old_user in old_users
				.iter()
				.filter(|old_user| !users.iter().any(|u| u.name == old_user.name))
			{
				self.user_manager
					.delete(&old_user.name)
					.map_err(|_| Error::Unspecified)?;
			}

			// Insert new users
			for new_user in users
				.iter()
				.filter(|u| !old_users.iter().any(|old_user| old_user.name == u.name))
			{
				self.user_manager.create(new_user)?;
			}

			// Update users
			for user in users {
				self.user_manager.set_password(&user.name, &user.password)?;
				self.user_manager.set_is_admin(&user.name, user.admin)?;
			}
		}

		Ok(())
	}
}

#[cfg(test)]
mod test {

	use super::*;
	use crate::app::test;
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
}
