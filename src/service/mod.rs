use std::fs;
use std::path::PathBuf;

use crate::app::{index::Index, lastfm, playlists, thumbnail, user, vfs};
use crate::config;
use crate::db::DB;

mod dto;
mod error;

#[cfg(test)]
mod test;

#[cfg(feature = "service-rocket")]
mod rocket;
#[cfg(feature = "service-rocket")]
pub use self::rocket::*;

pub struct Context {
	pub port: u16,
	pub auth_secret: Vec<u8>,
	pub web_dir_path: PathBuf,
	pub swagger_dir_path: PathBuf,
	pub web_url: String,
	pub swagger_url: String,
	pub api_url: String,
	pub db: DB,
	pub index: Index,
	pub lastfm_manager: lastfm::Manager,
	pub playlists_manager: playlists::Manager,
	pub thumbnail_manager: thumbnail::Manager,
	pub user_manager: user::Manager,
	pub vfs_manager: vfs::Manager,
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
		let db_path = self.database_file_path.unwrap_or_else(|| {
			let mut path = PathBuf::from(option_env!("POLARIS_DB_DIR").unwrap_or("."));
			path.push("db.sqlite");
			path
		});
		fs::create_dir_all(&db_path.parent().unwrap())?;
		let db = DB::new(&db_path)?;

		if let Some(config_path) = self.config_file_path {
			let config = config::parse_toml_file(&config_path)?;
			config::amend(&db, &config)?;
		}
		let auth_secret = config::get_auth_secret(&db)?;

		let web_dir_path = self
			.web_dir_path
			.or(option_env!("POLARIS_WEB_DIR").map(PathBuf::from))
			.unwrap_or([".", "web"].iter().collect());
		fs::create_dir_all(&web_dir_path)?;

		let swagger_dir_path = self
			.swagger_dir_path
			.or(option_env!("POLARIS_SWAGGER_DIR").map(PathBuf::from))
			.unwrap_or([".", "docs", "swagger"].iter().collect());
		fs::create_dir_all(&swagger_dir_path)?;

		let mut thumbnails_dir_path = self
			.cache_dir_path
			.or(option_env!("POLARIS_CACHE_DIR").map(PathBuf::from))
			.unwrap_or(PathBuf::from(".").to_owned());
		thumbnails_dir_path.push("thumbnails");

		let vfs_manager = vfs::Manager::new(db.clone());
		let index = Index::new(db.clone(), vfs_manager.clone());
		let playlists_manager = playlists::Manager::new(db.clone(), vfs_manager.clone());
		let thumbnail_manager = thumbnail::Manager::new(thumbnails_dir_path);
		let user_manager = user::Manager::new(db.clone());
		let lastfm_manager = lastfm::Manager::new(index.clone(), user_manager.clone());

		Ok(Context {
			port: self.port.unwrap_or(5050),
			auth_secret,
			api_url: "/api".to_owned(),
			swagger_url: "/swagger".to_owned(),
			web_url: "/".to_owned(),
			web_dir_path,
			swagger_dir_path,
			index,
			lastfm_manager,
			playlists_manager,
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
