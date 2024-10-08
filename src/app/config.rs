use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	sync::Arc,
	time::Duration,
};

use tokio::sync::RwLock;

use crate::app::Error;

mod mounts;
mod raw;
mod user;

pub use mounts::*;
pub use user::*;

use super::auth;

#[derive(Default)]
pub struct Config {
	pub reindex_every_n_seconds: Option<u64>,
	pub album_art_pattern: Option<String>,
	pub ddns_url: Option<String>,
	pub mount_dirs: Vec<MountDir>,
	pub users: HashMap<String, User>,
}

impl TryFrom<raw::Config> for Config {
	type Error = Error;

	fn try_from(raw: raw::Config) -> Result<Self, Self::Error> {
		let mut users: HashMap<String, User> = HashMap::new();
		for user in raw.users {
			if let Ok(user) = <raw::User as TryInto<User>>::try_into(user) {
				users.insert(user.name.clone(), user);
			}
		}

		let mount_dirs = raw
			.mount_dirs
			.into_iter()
			.filter_map(|m| m.try_into().ok())
			.collect();

		Ok(Config {
			reindex_every_n_seconds: raw.reindex_every_n_seconds, // TODO validate and warn
			album_art_pattern: raw.album_art_pattern,             // TODO validate and warn
			ddns_url: raw.ddns_url,                               // TODO validate and warn
			mount_dirs,
			users,
		})
	}
}

#[derive(Clone)]
pub struct Manager {
	config_file_path: PathBuf,
	config: Arc<tokio::sync::RwLock<Config>>,
	auth_secret: auth::Secret,
}

impl Manager {
	pub async fn new(config_file_path: &Path, auth_secret: auth::Secret) -> Result<Self, Error> {
		let raw_config = raw::Config::default(); // TODO read from disk!!
		let config = raw_config.try_into()?;
		let manager = Self {
			config_file_path: config_file_path.to_owned(),
			config: Arc::new(RwLock::new(config)),
			auth_secret,
		};
		Ok(manager)
	}

	pub async fn apply(&self, raw_config: raw::Config) -> Result<(), Error> {
		*self.config.write().await = raw_config.try_into()?;
		// TODO persistence
		Ok(())
	}

	pub async fn get_index_sleep_duration(&self) -> Duration {
		let config = self.config.read().await;
		let seconds = config.reindex_every_n_seconds.unwrap_or(1800);
		Duration::from_secs(seconds)
	}

	pub async fn get_index_album_art_pattern(&self) -> String {
		let config = self.config.read().await;
		let pattern = config.album_art_pattern.clone();
		pattern.unwrap_or("Folder.(jpeg|jpg|png)".to_owned())
	}

	pub async fn get_ddns_update_url(&self) -> Option<String> {
		self.config.read().await.ddns_url.clone()
	}

	pub async fn get_users(&self) -> Vec<User> {
		self.config.read().await.users.values().cloned().collect()
	}

	pub async fn get_user(&self, username: &str) -> Result<User, Error> {
		let config = self.config.read().await;
		let user = config.users.get(username);
		user.cloned().ok_or(Error::UserNotFound)
	}

	pub async fn create_user(
		&self,
		username: &str,
		password: &str,
		admin: bool,
	) -> Result<(), Error> {
		let mut config = self.config.write().await;
		config.create_user(username, password, admin)
		// TODO persistence
	}

	pub async fn login(&self, username: &str, password: &str) -> Result<auth::Token, Error> {
		let config = self.config.read().await;
		config.login(username, password, &self.auth_secret)
	}

	pub async fn set_is_admin(&self, username: &str, is_admin: bool) -> Result<(), Error> {
		let mut config = self.config.write().await;
		config.set_is_admin(username, is_admin)
		// TODO persistence
	}

	pub async fn set_password(&self, username: &str, password: &str) -> Result<(), Error> {
		let mut config = self.config.write().await;
		config.set_password(username, password)
		// TODO persistence
	}

	pub async fn authenticate(
		&self,
		auth_token: &auth::Token,
		scope: auth::Scope,
	) -> Result<auth::Authorization, Error> {
		let config = self.config.read().await;
		config.authenticate(auth_token, scope, &self.auth_secret)
	}

	pub async fn delete_user(&self, username: &str) {
		let mut config = self.config.write().await;
		config.delete_user(username);
		// TODO persistence
	}

	pub async fn get_mounts(&self) -> Vec<MountDir> {
		self.config.read().await.mount_dirs.clone()
	}

	pub async fn resolve_virtual_path<P: AsRef<Path>>(
		&self,
		virtual_path: P,
	) -> Result<PathBuf, Error> {
		let config = self.config.read().await;
		config.resolve_virtual_path(virtual_path)
	}

	pub async fn set_mounts(&self, mount_dirs: Vec<raw::MountDir>) {
		self.config.write().await.set_mounts(mount_dirs);
		// TODO persistence
	}
}

#[cfg(test)]
mod test {

	use super::*;
	use crate::app::test;
	use crate::test_name;

	#[tokio::test]
	async fn can_apply_config() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;
		let new_config = raw::Config {
			reindex_every_n_seconds: Some(100),
			album_art_pattern: Some("cool_pattern".to_owned()),
			mount_dirs: vec![raw::MountDir {
				source: "/home/music".to_owned(),
				name: "Library".to_owned(),
			}],
			ddns_url: Some("https://cooldns.com".to_owned()),
			users: vec![],
		};
		ctx.config_manager.apply(new_config.clone()).await.unwrap();
		assert_eq!(new_config, ctx.config_manager.config.read().await.clone(),);
	}
}
