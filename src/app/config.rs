use serde::Deserialize;
use std::io::Read;
use std::path;

use crate::app::{ddns, settings, user, vfs};

mod error;
#[cfg(test)]
mod test;

pub use error::*;

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
			self.settings_manager
				.amend(new_settings)
				.map_err(|_| Error::Unspecified)?;
		}

		if let Some(mount_dirs) = &config.mount_dirs {
			self.vfs_manager
				.set_mount_dirs(mount_dirs)
				.map_err(|_| Error::Unspecified)?;
		}

		if let Some(ddns_config) = &config.ydns {
			self.ddns_manager
				.set_config(ddns_config)
				.map_err(|_| Error::Unspecified)?;
		}

		if let Some(ref users) = config.users {
			let old_users: Vec<user::User> =
				self.user_manager.list().map_err(|_| Error::Unspecified)?;

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
				self.user_manager
					.create(new_user)
					.map_err(|_| Error::Unspecified)?;
			}

			// Update users
			for user in users {
				self.user_manager
					.set_password(&user.name, &user.password)
					.map_err(|_| Error::Unspecified)?;
				self.user_manager
					.set_is_admin(&user.name, user.admin)
					.map_err(|_| Error::Unspecified)?;
			}
		}

		Ok(())
	}
}
