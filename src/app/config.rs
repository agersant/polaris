use std::{
	path::{Path, PathBuf},
	sync::Arc,
	time::Duration,
};

use log::{error, info};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_full::{Debouncer, FileIdMap};
use regex::Regex;
use tokio::sync::{futures::Notified, Notify, RwLock};

use crate::app::Error;

mod mounts;
pub mod storage;
mod user;

pub use mounts::*;
pub use user::*;

use super::auth;

#[derive(Debug, Clone, Default)]
pub struct Config {
	pub album_art_pattern: Option<Regex>,
	pub ddns_update_url: Option<http::Uri>,
	pub mount_dirs: Vec<MountDir>,
	pub users: Vec<User>,
}

impl TryFrom<storage::Config> for Config {
	type Error = Error;

	fn try_from(c: storage::Config) -> Result<Self, Self::Error> {
		let mut config = Config::default();
		config.set_mounts(c.mount_dirs)?;
		config.set_users(c.users)?;

		config.album_art_pattern = match c.album_art_pattern.as_deref().map(Regex::new) {
			Some(Ok(u)) => Some(u),
			Some(Err(_)) => return Err(Error::IndexAlbumArtPatternInvalid),
			None => None,
		};

		config.ddns_update_url = match c.ddns_update_url.map(http::Uri::try_from) {
			Some(Ok(u)) => Some(u),
			Some(Err(_)) => return Err(Error::DDNSUpdateURLInvalid),
			None => None,
		};

		Ok(config)
	}
}

impl From<Config> for storage::Config {
	fn from(c: Config) -> Self {
		Self {
			album_art_pattern: c.album_art_pattern.map(|p| p.as_str().to_owned()),
			mount_dirs: c.mount_dirs.into_iter().map(|d| d.into()).collect(),
			ddns_update_url: c.ddns_update_url.map(|u| u.to_string()),
			users: c.users.into_iter().map(|u| u.into()).collect(),
		}
	}
}

#[derive(Clone)]
pub struct Manager {
	config_file_path: PathBuf,
	config: Arc<RwLock<Config>>,
	auth_secret: auth::Secret,
	#[allow(dead_code)]
	file_watcher: Arc<Debouncer<RecommendedWatcher, FileIdMap>>,
	change_notify: Arc<Notify>,
}

impl Manager {
	pub async fn new(config_file_path: &Path, auth_secret: auth::Secret) -> Result<Self, Error> {
		if let Some(parent) = config_file_path.parent() {
			tokio::fs::create_dir_all(parent)
				.await
				.map_err(|e| Error::Io(parent.to_owned(), e))?;
		}

		match tokio::fs::File::create_new(config_file_path).await {
			Ok(_) => (),
			Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
			Err(e) => {
				error!("Failed to create config file at {config_file_path:#?}: {e}");
				return Err(Error::Io(config_file_path.to_owned(), e));
			}
		};

		let notify = Arc::new(Notify::new());
		let mut debouncer = notify_debouncer_full::new_debouncer(Duration::from_secs(1), None, {
			let notify = notify.clone();
			move |_| {
				notify.notify_waiters();
			}
		})?;

		debouncer
			.watcher()
			.watch(config_file_path, RecursiveMode::NonRecursive)?;

		let manager = Self {
			config_file_path: config_file_path.to_owned(),
			config: Arc::new(RwLock::new(Config::default())),
			auth_secret,
			file_watcher: Arc::new(debouncer),
			change_notify: Arc::default(),
		};

		tokio::task::spawn({
			let manager = manager.clone();
			async move {
				loop {
					notify.notified().await;
					if let Err(e) = manager.reload_config().await {
						error!("Configuration error: {e}");
					} else {
						info!("Successfully applied configuration change");
					}
				}
			}
		});

		manager.reload_config().await?;

		Ok(manager)
	}

	pub fn on_config_change(&self) -> Notified {
		self.change_notify.notified()
	}

	async fn reload_config(&self) -> Result<(), Error> {
		let config = Self::read_config(&self.config_file_path).await?;
		self.apply_config(config).await
	}

	async fn read_config(config_file_path: &Path) -> Result<storage::Config, Error> {
		let config_content = tokio::fs::read_to_string(config_file_path)
			.await
			.map_err(|e| Error::Io(config_file_path.to_owned(), e))?;
		toml::de::from_str::<storage::Config>(&config_content).map_err(Error::ConfigDeserialization)
	}

	pub async fn save_config(&self) -> Result<(), Error> {
		let serialized = toml::ser::to_string_pretty::<storage::Config>(
			&self.config.read().await.clone().into(),
		)
		.map_err(Error::ConfigSerialization)?;
		tokio::fs::write(&self.config_file_path, serialized.as_bytes())
			.await
			.map_err(|e| Error::Io(self.config_file_path.clone(), e))?;
		Ok(())
	}

	pub async fn apply_config(&self, new_config: storage::Config) -> Result<(), Error> {
		let mut config = self.config.write().await;
		*config = new_config.try_into()?;
		self.change_notify.notify_waiters();
		Ok(())
	}

	async fn mutate<F: FnOnce(&mut Config)>(&self, op: F) -> Result<(), Error> {
		self.mutate_fallible(|c| {
			op(c);
			Ok(())
		})
		.await
	}

	async fn mutate_fallible<F: FnOnce(&mut Config) -> Result<(), Error>>(
		&self,
		op: F,
	) -> Result<(), Error> {
		{
			let mut config = self.config.write().await;
			op(&mut config)?;
		}
		self.change_notify.notify_waiters();
		self.save_config().await?;
		Ok(())
	}

	pub async fn get_index_album_art_pattern(&self) -> Regex {
		let config = self.config.read().await;
		let pattern = config.album_art_pattern.clone();
		pattern.unwrap_or_else(|| Regex::new("Folder.(jpeg|jpg|png)").unwrap())
	}

	pub async fn set_index_album_art_pattern(&self, regex: Regex) -> Result<(), Error> {
		self.mutate(|c| {
			c.album_art_pattern = Some(regex);
		})
		.await
	}

	pub async fn get_ddns_update_url(&self) -> Option<http::Uri> {
		self.config.read().await.ddns_update_url.clone()
	}

	pub async fn set_ddns_update_url(&self, url: Option<http::Uri>) -> Result<(), Error> {
		self.mutate(|c| {
			c.ddns_update_url = url;
		})
		.await
	}

	pub async fn get_users(&self) -> Vec<User> {
		self.config.read().await.users.to_vec()
	}

	pub async fn get_user(&self, username: &str) -> Result<User, Error> {
		let config = self.config.read().await;
		config
			.get_user(username)
			.cloned()
			.ok_or(Error::UserNotFound)
	}

	pub async fn create_user(
		&self,
		username: &str,
		password: &str,
		admin: bool,
	) -> Result<(), Error> {
		self.mutate_fallible(|c| c.create_user(username, password, admin))
			.await
	}

	pub async fn login(&self, username: &str, password: &str) -> Result<auth::Token, Error> {
		let config = self.config.read().await;
		config.login(username, password, &self.auth_secret)
	}

	pub async fn set_is_admin(&self, username: &str, is_admin: bool) -> Result<(), Error> {
		self.mutate_fallible(|c| c.set_is_admin(username, is_admin))
			.await
	}

	pub async fn set_password(&self, username: &str, password: &str) -> Result<(), Error> {
		self.mutate_fallible(|c| c.set_password(username, password))
			.await
	}

	pub async fn authenticate(
		&self,
		auth_token: &auth::Token,
		scope: auth::Scope,
	) -> Result<auth::Authorization, Error> {
		let config = self.config.read().await;
		config.authenticate(auth_token, scope, &self.auth_secret)
	}

	pub async fn delete_user(&self, username: &str) -> Result<(), Error> {
		self.mutate(|c| c.delete_user(username)).await
	}

	pub async fn get_mounts(&self) -> Vec<MountDir> {
		let config = self.config.read().await;
		config.mount_dirs.to_vec()
	}

	pub async fn resolve_virtual_path<P: AsRef<Path>>(
		&self,
		virtual_path: P,
	) -> Result<PathBuf, Error> {
		let config = self.config.read().await;
		config.resolve_virtual_path(virtual_path)
	}

	pub async fn set_mounts(&self, mount_dirs: Vec<storage::MountDir>) -> Result<(), Error> {
		self.mutate_fallible(|c| c.set_mounts(mount_dirs)).await
	}
}

#[cfg(test)]
mod test {
	use crate::app::test;
	use crate::test_name;

	use super::*;

	#[tokio::test]
	async fn blank_config_round_trip() {
		let config_path = PathBuf::from_iter(["test-data", "blank.toml"]);
		let manager = Manager::new(&config_path, auth::Secret([0; 32]))
			.await
			.unwrap();
		let config: storage::Config = manager.config.read().await.clone().into();
		assert_eq!(config, storage::Config::default());
	}

	#[tokio::test]
	async fn can_read_config() {
		let config_path = PathBuf::from_iter(["test-data", "config.toml"]);
		let manager = Manager::new(&config_path, auth::Secret([0; 32]))
			.await
			.unwrap();
		let config: storage::Config = manager.config.read().await.clone().into();

		assert_eq!(
			config.album_art_pattern,
			Some(r#"^Folder\.(png|jpg|jpeg)$"#.to_owned())
		);
		assert_eq!(
			config.mount_dirs,
			vec![storage::MountDir {
				source: PathBuf::from("test-data/small-collection"),
				name: "root".to_owned(),
			}]
		);
		assert_eq!(config.users[0].name, "test_user");
		assert_eq!(config.users[0].admin, Some(true));
		assert_eq!(
			config.users[0].initial_password,
			Some("very_secret_password".to_owned())
		);
		assert!(config.users[0].hashed_password.is_some());
	}

	#[tokio::test]
	async fn can_write_config() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;
		ctx.config_manager
			.create_user("Walter", "example_password", false)
			.await
			.unwrap();

		let manager = Manager::new(&ctx.config_manager.config_file_path, auth::Secret([0; 32]))
			.await
			.unwrap();
		assert!(manager.get_user("Walter").await.is_ok());
	}
}
