use std::fs;
use std::path::PathBuf;

use crate::app::{config, ddns, index::Index, lastfm, playlist, settings, thumbnail, user, vfs};
use crate::db::DB;

mod dto;
mod error;

#[cfg(test)]
mod test;

mod actix;
pub use actix::*;

#[derive(Clone)]
pub struct Context {
	pub port: u16,
	pub auth_secret: settings::AuthSecret,
	pub web_dir_path: PathBuf,
	pub swagger_dir_path: PathBuf,
	pub web_url: String,
	pub swagger_url: String,
	pub api_url: String,
	pub db: DB,
	pub index: Index,
	pub config_manager: config::Manager,
	pub ddns_manager: ddns::Manager,
	pub lastfm_manager: lastfm::Manager,
	pub playlist_manager: playlist::Manager,
	pub settings_manager: settings::Manager,
	pub thumbnail_manager: thumbnail::Manager,
	pub user_manager: user::Manager,
	pub vfs_manager: vfs::Manager,
}

struct Paths {
	db_dir_path: PathBuf,
	web_dir_path: PathBuf,
	swagger_dir_path: PathBuf,
	cache_dir_path: PathBuf,
}

// TODO Make this the only implementation when we can expand %LOCALAPPDATA% correctly on Windows
// And fix the installer accordingly (`release_script.ps1`)
#[cfg(not(windows))]
impl Default for Paths {
	fn default() -> Self {
		Self {
			db_dir_path: ["."].iter().collect(),
			web_dir_path: [".", "web"].iter().collect(),
			swagger_dir_path: [".", "docs", "swagger"].iter().collect(),
			cache_dir_path: ["."].iter().collect(),
		}
	}
}

#[cfg(windows)]
impl Default for Paths {
	fn default() -> Self {
		let local_app_data = std::env::var("LOCALAPPDATA").map(PathBuf::from).unwrap();
		let install_directory: PathBuf =
			local_app_data.join(["Permafrost", "Polaris"].iter().collect::<PathBuf>());
		Self {
			db_dir_path: install_directory.clone(),
			web_dir_path: install_directory.join("web"),
			swagger_dir_path: install_directory.join("swagger"),
			cache_dir_path: install_directory.clone(),
		}
	}
}

impl Paths {
	fn new() -> Self {
		let defaults = Self::default();
		Self {
			db_dir_path: option_env!("POLARIS_DB_DIR")
				.map(PathBuf::from)
				.unwrap_or(defaults.db_dir_path),
			web_dir_path: option_env!("POLARIS_WEB_DIR")
				.map(PathBuf::from)
				.unwrap_or(defaults.web_dir_path),
			swagger_dir_path: option_env!("POLARIS_SWAGGER_DIR")
				.map(PathBuf::from)
				.unwrap_or(defaults.swagger_dir_path),
			cache_dir_path: option_env!("POLARIS_CACHE_DIR")
				.map(PathBuf::from)
				.unwrap_or(defaults.cache_dir_path),
		}
	}
}

pub struct ContextBuilder {
	port: Option<u16>,
	config_file_path: Option<PathBuf>,
	database_file_path: Option<PathBuf>,
	web_dir_path: Option<PathBuf>,
	swagger_dir_path: Option<PathBuf>,
	cache_dir_path: Option<PathBuf>,
}

impl ContextBuilder {
	pub fn new() -> Self {
		Self {
			port: None,
			config_file_path: None,
			database_file_path: None,
			web_dir_path: None,
			swagger_dir_path: None,
			cache_dir_path: None,
		}
	}

	pub fn build(self) -> anyhow::Result<Context> {
		let paths = Paths::new();

		let db_path = self
			.database_file_path
			.unwrap_or(paths.db_dir_path.join("db.sqlite"));
		fs::create_dir_all(&db_path.parent().unwrap())?;
		let db = DB::new(&db_path)?;

		let web_dir_path = self.web_dir_path.unwrap_or(paths.web_dir_path);
		fs::create_dir_all(&web_dir_path)?;

		let swagger_dir_path = self.swagger_dir_path.unwrap_or(paths.swagger_dir_path);
		fs::create_dir_all(&swagger_dir_path)?;

		let thumbnails_dir_path = self
			.cache_dir_path
			.unwrap_or(paths.cache_dir_path)
			.join("thumbnails");

		let vfs_manager = vfs::Manager::new(db.clone());
		let settings_manager = settings::Manager::new(db.clone());
		let auth_secret = settings_manager.get_auth_secret()?;
		let ddns_manager = ddns::Manager::new(db.clone());
		let user_manager = user::Manager::new(db.clone(), auth_secret);
		let index = Index::new(db.clone(), vfs_manager.clone(), settings_manager.clone());
		let config_manager = config::Manager::new(
			settings_manager.clone(),
			user_manager.clone(),
			vfs_manager.clone(),
			ddns_manager.clone(),
		);
		let playlist_manager = playlist::Manager::new(db.clone(), vfs_manager.clone());
		let thumbnail_manager = thumbnail::Manager::new(thumbnails_dir_path);
		let lastfm_manager = lastfm::Manager::new(index.clone(), user_manager.clone());

		if let Some(config_path) = self.config_file_path {
			let config = config::Config::from_path(&config_path)?;
			config_manager.apply(&config)?;
		}

		let auth_secret = settings_manager.get_auth_secret()?;

		Ok(Context {
			port: self.port.unwrap_or(5050),
			auth_secret,
			api_url: "/api".to_owned(),
			swagger_url: "/swagger".to_owned(),
			web_url: "/".to_owned(),
			web_dir_path,
			swagger_dir_path,
			index,
			config_manager,
			ddns_manager,
			lastfm_manager,
			playlist_manager,
			settings_manager,
			thumbnail_manager,
			user_manager,
			vfs_manager,
			db,
		})
	}

	pub fn port(mut self, port: u16) -> Self {
		self.port = Some(port);
		self
	}

	pub fn config_file_path(mut self, path: PathBuf) -> Self {
		self.config_file_path = Some(path);
		self
	}

	pub fn database_file_path(mut self, path: PathBuf) -> Self {
		self.database_file_path = Some(path);
		self
	}

	pub fn web_dir_path(mut self, path: PathBuf) -> Self {
		self.web_dir_path = Some(path);
		self
	}

	pub fn swagger_dir_path(mut self, path: PathBuf) -> Self {
		self.swagger_dir_path = Some(path);
		self
	}

	pub fn cache_dir_path(mut self, path: PathBuf) -> Self {
		self.cache_dir_path = Some(path);
		self
	}
}
