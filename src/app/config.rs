use std::{
	io::Read,
	ops::Deref,
	path::{Path, PathBuf},
	sync::Arc,
	time::Duration,
};

use pbkdf2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use pbkdf2::Pbkdf2;
use rand::rngs::OsRng;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::app::Error;

#[derive(Clone, Default)]
pub struct AuthSecret {
	pub key: [u8; 32],
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct MountDir {
	pub source: String,
	pub name: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct User {
	pub name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub admin: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub initial_password: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub hashed_password: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Config {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub reindex_every_n_seconds: Option<u64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub album_art_pattern: Option<String>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub mount_dirs: Vec<MountDir>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ddns_url: Option<String>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub users: Vec<User>,
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
	config_file_path: PathBuf,
	config: Arc<tokio::sync::RwLock<Config>>,
}

impl Manager {
	pub async fn new(config_file_path: &Path) -> Result<Self, Error> {
		let config = Config::default(); // TODO read from disk!!
		let manager = Self {
			config_file_path: config_file_path.to_owned(),
			config: Arc::default(),
		};
		manager.apply(config);
		Ok(manager)
	}

	pub async fn apply(&self, mut config: Config) -> Result<(), Error> {
		config
			.users
			.retain(|u| u.initial_password.is_some() || u.hashed_password.is_some());

		for user in &mut config.users {
			if let (Some(password), None) = (&user.initial_password, &user.hashed_password) {
				user.hashed_password = Some(hash_password(&password)?);
			}
		}

		*self.config.write().await = config;

		// TODO persistence

		Ok(())
	}

	pub async fn get_index_sleep_duration(&self) -> Duration {
		let seconds = self
			.config
			.read()
			.await
			.reindex_every_n_seconds
			.unwrap_or(1800);
		Duration::from_secs(seconds)
	}

	pub async fn get_index_album_art_pattern(&self) -> String {
		self.config
			.read()
			.await
			.album_art_pattern
			.clone()
			.unwrap_or("Folder.(jpeg|jpg|png)".to_owned())
	}

	pub async fn get_ddns_update_url(&self) -> Option<String> {
		self.config.read().await.ddns_url.clone()
	}

	pub async fn get_users(&self) -> Vec<User> {
		self.config.read().await.users.clone()
	}

	pub async fn get_user(&self, username: &str) -> Result<User, Error> {
		let config = self.config.read().await;
		let user = config.users.iter().find(|u| u.name == username);
		user.cloned().ok_or(Error::UserNotFound)
	}

	pub async fn delete_user(&self, username: &str) {
		let mut config = self.config.write().await;
		config.users.retain(|u| u.name != username);
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
		for mount in &config.mount_dirs {
			let mounth_source = sanitize_path(&mount.source);
			let mount_path = Path::new(&mount.name);
			if let Ok(p) = virtual_path.as_ref().strip_prefix(mount_path) {
				return if p.components().count() == 0 {
					Ok(mounth_source)
				} else {
					Ok(mounth_source.join(p))
				};
			}
		}
		Err(Error::CouldNotMapToRealPath(virtual_path.as_ref().into()))
	}

	pub async fn set_mounts(&self, mount_dirs: Vec<MountDir>) {
		self.config.write().await.mount_dirs = mount_dirs;
		// TODO persistence
	}
}

fn sanitize_path(source: &str) -> PathBuf {
	let separator_regex = Regex::new(r"\\|/").unwrap();
	let mut correct_separator = String::new();
	correct_separator.push(std::path::MAIN_SEPARATOR);
	let path_string = separator_regex.replace_all(source, correct_separator.as_str());
	PathBuf::from(path_string.deref())
}

fn hash_password(password: &str) -> Result<String, Error> {
	if password.is_empty() {
		return Err(Error::EmptyPassword);
	}
	let salt = SaltString::generate(&mut OsRng);
	match Pbkdf2.hash_password(password.as_bytes(), &salt) {
		Ok(h) => Ok(h.to_string()),
		Err(_) => Err(Error::PasswordHashing),
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
		let new_config = Config {
			reindex_every_n_seconds: Some(100),
			album_art_pattern: Some("cool_pattern".to_owned()),
			mount_dirs: vec![MountDir {
				source: "/home/music".to_owned(),
				name: "Library".to_owned(),
			}],
			ddns_url: Some("https://cooldns.com".to_owned()),
			users: vec![],
		};
		ctx.config_manager.apply(new_config.clone()).await.unwrap();
		assert_eq!(new_config, ctx.config_manager.config.read().await.clone(),);
	}

	#[tokio::test]
	async fn applying_config_adds_or_preserves_password_hashes() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;

		let new_config = Config {
			users: vec![
				User {
					name: "walter".to_owned(),
					initial_password: Some("super salmon 64".to_owned()),
					..Default::default()
				},
				User {
					name: "lara".to_owned(),
					hashed_password: Some("hash".to_owned()),
					..Default::default()
				},
			],
			..Default::default()
		};

		ctx.config_manager.apply(new_config).await.unwrap();
		let actual_config = ctx.config_manager.config.read().await.clone();

		assert_eq!(actual_config.users[0].name, "walter");
		assert_eq!(
			actual_config.users[0].initial_password,
			Some("super salmon 64".to_owned())
		);
		assert!(actual_config.users[0].hashed_password.is_some());

		assert_eq!(
			actual_config.users[1],
			User {
				name: "lara".to_owned(),
				hashed_password: Some("hash".to_owned()),
				..Default::default()
			}
		);
	}

	#[test]
	fn converts_virtual_to_real() {
		let vfs = VFS::new(vec![Mount {
			name: "root".to_owned(),
			source: Path::new("test_dir").to_owned(),
		}]);
		let real_path: PathBuf = ["test_dir", "somewhere", "something.png"].iter().collect();
		let virtual_path: PathBuf = ["root", "somewhere", "something.png"].iter().collect();
		let converted_path = vfs.virtual_to_real(virtual_path.as_path()).unwrap();
		assert_eq!(converted_path, real_path);
	}

	#[test]
	fn converts_virtual_to_real_top_level() {
		let vfs = VFS::new(vec![Mount {
			name: "root".to_owned(),
			source: Path::new("test_dir").to_owned(),
		}]);
		let real_path = Path::new("test_dir");
		let converted_path = vfs.virtual_to_real(Path::new("root")).unwrap();
		assert_eq!(converted_path, real_path);
	}

	#[test]
	fn cleans_path_string() {
		let mut correct_path = path::PathBuf::new();
		if cfg!(target_os = "windows") {
			correct_path.push("C:\\");
		} else {
			correct_path.push("/usr");
		}
		correct_path.push("some");
		correct_path.push("path");

		let tests = if cfg!(target_os = "windows") {
			vec![
				r#"C:/some/path"#,
				r#"C:\some\path"#,
				r#"C:\some\path\"#,
				r#"C:\some\path\\\\"#,
				r#"C:\some/path//"#,
			]
		} else {
			vec![
				r#"/usr/some/path"#,
				r#"/usr\some\path"#,
				r#"/usr\some\path\"#,
				r#"/usr\some\path\\\\"#,
				r#"/usr\some/path//"#,
			]
		};

		for test in tests {
			let mount_dir = MountDir {
				source: test.to_owned(),
				name: "name".to_owned(),
			};
			let mount: Mount = mount_dir.into();
			assert_eq!(mount.source, correct_path);
		}
	}
}
