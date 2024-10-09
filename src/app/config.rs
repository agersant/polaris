use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	sync::Arc,
	time::Duration,
};

use regex::Regex;
use tokio::sync::RwLock;

use crate::app::Error;

mod mounts;
pub mod storage;
mod user;

pub use mounts::*;
pub use user::*;

use super::auth;

#[derive(Debug, Clone, Default)]
pub struct Config {
	pub reindex_every_n_seconds: Option<u64>,
	pub album_art_pattern: Option<Regex>,
	pub ddns_url: Option<http::Uri>,
	pub mount_dirs: Vec<MountDir>,
	pub users: HashMap<String, User>,
}

impl TryFrom<storage::Config> for Config {
	type Error = Error;

	fn try_from(c: storage::Config) -> Result<Self, Self::Error> {
		let mut users: HashMap<String, User> = HashMap::new();
		for user in c.users {
			if let Ok(user) = <storage::User as TryInto<User>>::try_into(user) {
				users.insert(user.name.clone(), user);
			}
		}

		let mount_dirs = c
			.mount_dirs
			.into_iter()
			.filter_map(|m| m.try_into().ok())
			.collect();

		let ddns_url = match c.ddns_url.map(http::Uri::try_from) {
			Some(Ok(u)) => Some(u),
			Some(Err(_)) => return Err(Error::DDNSUpdateURLInvalid),
			None => None,
		};

		let album_art_pattern = match c.album_art_pattern.as_deref().map(Regex::new) {
			Some(Ok(u)) => Some(u),
			Some(Err(_)) => return Err(Error::IndexAlbumArtPatternInvalid),
			None => None,
		};

		Ok(Config {
			reindex_every_n_seconds: c.reindex_every_n_seconds, // TODO validate and warn
			album_art_pattern,
			ddns_url,
			mount_dirs,
			users,
		})
	}
}

impl From<Config> for storage::Config {
	fn from(c: Config) -> Self {
		Self {
			reindex_every_n_seconds: c.reindex_every_n_seconds,
			album_art_pattern: c.album_art_pattern.map(|p| p.as_str().to_owned()),
			mount_dirs: c.mount_dirs.into_iter().map(|d| d.into()).collect(),
			ddns_url: c.ddns_url.map(|u| u.to_string()),
			users: c.users.into_iter().map(|(_, u)| u.into()).collect(),
		}
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
		let config = {
			if tokio::fs::try_exists(config_file_path)
				.await
				.map_err(|e| Error::Io(config_file_path.to_owned(), e))?
			{
				let config_content = tokio::fs::read_to_string(config_file_path)
					.await
					.map_err(|e| Error::Io(config_file_path.to_owned(), e))?;
				let config = toml::de::from_str::<storage::Config>(&config_content)
					.map_err(Error::ConfigDeserialization)?;
				config.try_into()?
			} else {
				Config::default()
			}
		};

		let manager = Self {
			config_file_path: config_file_path.to_owned(),
			config: Arc::new(RwLock::new(config)),
			auth_secret,
		};

		Ok(manager)
	}

	pub async fn apply(&self, config: storage::Config) -> Result<(), Error> {
		*self.config.write().await = config.try_into()?;
		// TODO persistence
		Ok(())
	}

	pub async fn get_index_sleep_duration(&self) -> Duration {
		let config = self.config.read().await;
		let seconds = config.reindex_every_n_seconds.unwrap_or(1800);
		Duration::from_secs(seconds)
	}

	pub async fn set_index_sleep_duration(&self, duration: Duration) {
		let mut config = self.config.write().await;
		config.reindex_every_n_seconds = Some(duration.as_secs());
		// TODO persistence
	}

	pub async fn get_index_album_art_pattern(&self) -> Regex {
		let config = self.config.read().await;
		let pattern = config.album_art_pattern.clone();
		pattern.unwrap_or_else(|| Regex::new("Folder.(jpeg|jpg|png)").unwrap())
	}

	pub async fn set_index_album_art_pattern(&self, regex: Regex) {
		let mut config = self.config.write().await;
		config.album_art_pattern = Some(regex);
		// TODO persistence
	}

	pub async fn get_ddns_update_url(&self) -> Option<http::Uri> {
		self.config.read().await.ddns_url.clone()
	}

	pub async fn set_ddns_update_url(&self, url: http::Uri) {
		let mut config = self.config.write().await;
		config.ddns_url = Some(url);
		// TODO persistence
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

	pub async fn set_mounts(&self, mount_dirs: Vec<storage::MountDir>) {
		self.config.write().await.set_mounts(mount_dirs);
		// TODO persistence
	}
}

#[cfg(test)]
mod test {

	use std::path::PathBuf;

	use crate::app::config::storage::*;
	use crate::app::test;
	use crate::test_name;

	#[tokio::test]
	async fn can_apply_config() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;
		let new_config = Config {
			reindex_every_n_seconds: Some(100),
			album_art_pattern: Some("cool_pattern".to_owned()),
			mount_dirs: vec![MountDir {
				source: PathBuf::from("/home/music"),
				name: "Library".to_owned(),
			}],
			ddns_url: Some("https://cooldns.com".to_owned()),
			users: vec![],
		};
		ctx.config_manager.apply(new_config.clone()).await.unwrap();
		let effective_config: Config = ctx.config_manager.config.read().await.clone().into();
		assert_eq!(new_config, effective_config,);
	}
}
