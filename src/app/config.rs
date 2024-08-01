use serde::Deserialize;
use std::{io::Read, path::Path};

use crate::app::{ddns, settings, user, vfs, Error};

#[derive(Default, Deserialize)]
pub struct Config {
	pub settings: Option<settings::NewSettings>,
	pub mount_dirs: Option<Vec<vfs::MountDir>>,
	pub ydns: Option<ddns::Config>,
	pub users: Option<Vec<user::NewUser>>,
}

impl Config {
	pub fn from_path(path: &Path) -> Result<Config, Error> {
		let mut config_file =
			std::fs::File::open(path).map_err(|e| Error::Io(path.to_owned(), e))?;
		let mut config_file_content = String::new();
		config_file
			.read_to_string(&mut config_file_content)
			.map_err(|e| Error::Io(path.to_owned(), e))?;
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

	pub async fn apply(&self, config: &Config) -> Result<(), Error> {
		if let Some(new_settings) = &config.settings {
			self.settings_manager.amend(new_settings).await?;
		}

		if let Some(mount_dirs) = &config.mount_dirs {
			self.vfs_manager.set_mount_dirs(mount_dirs).await?;
		}

		if let Some(ddns_config) = &config.ydns {
			self.ddns_manager.set_config(ddns_config).await?;
		}

		if let Some(ref users) = config.users {
			let old_users: Vec<user::User> = self.user_manager.list().await?;

			// Delete users that are not in new list
			for old_user in old_users
				.iter()
				.filter(|old_user| !users.iter().any(|u| u.name == old_user.name))
			{
				self.user_manager.delete(&old_user.name).await?;
			}

			// Insert new users
			for new_user in users
				.iter()
				.filter(|u| !old_users.iter().any(|old_user| old_user.name == u.name))
			{
				self.user_manager.create(new_user).await?;
			}

			// Update users
			for user in users {
				self.user_manager
					.set_password(&user.name, &user.password)
					.await?;
				self.user_manager
					.set_is_admin(&user.name, user.admin)
					.await?;
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

	#[tokio::test]
	async fn apply_saves_misc_settings() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;
		let new_config = Config {
			settings: Some(settings::NewSettings {
				album_art_pattern: Some("🖼️\\.jpg".into()),
				reindex_every_n_seconds: Some(100),
			}),
			..Default::default()
		};

		ctx.config_manager.apply(&new_config).await.unwrap();
		let settings = ctx.settings_manager.read().await.unwrap();
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

	#[tokio::test]
	async fn apply_saves_mount_points() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;

		let new_config = Config {
			mount_dirs: Some(vec![vfs::MountDir {
				source: "/home/music".into(),
				name: "🎵📁".into(),
			}]),
			..Default::default()
		};

		ctx.config_manager.apply(&new_config).await.unwrap();
		let actual_mount_dirs: Vec<vfs::MountDir> = ctx.vfs_manager.mount_dirs().await.unwrap();
		assert_eq!(actual_mount_dirs, new_config.mount_dirs.unwrap());
	}

	#[tokio::test]
	async fn apply_saves_ddns_settings() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;

		let new_config = Config {
			ydns: Some(ddns::Config {
				ddns_host: "🐸🐸🐸.ydns.eu".into(),
				ddns_username: "kfr🐸g".into(),
				ddns_password: "tasty🐞".into(),
			}),
			..Default::default()
		};

		ctx.config_manager.apply(&new_config).await.unwrap();
		let actual_ddns = ctx.ddns_manager.config().await.unwrap();
		assert_eq!(actual_ddns, new_config.ydns.unwrap());
	}

	#[tokio::test]
	async fn apply_can_toggle_admin() {
		let ctx = test::ContextBuilder::new(test_name!())
			.user("Walter", "Tasty🍖", true)
			.build()
			.await;

		assert!(ctx.user_manager.list().await.unwrap()[0].is_admin());

		let new_config = Config {
			users: Some(vec![user::NewUser {
				name: "Walter".into(),
				password: "Tasty🍖".into(),
				admin: false,
			}]),
			..Default::default()
		};
		ctx.config_manager.apply(&new_config).await.unwrap();
		assert!(!ctx.user_manager.list().await.unwrap()[0].is_admin());
	}
}
