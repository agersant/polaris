use crate::db::DB;
use crate::index::Index;
use crate::thumbnails::ThumbnailsManager;
use std::fs;
use std::path::PathBuf;

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
	pub thumbnails_manager: ThumbnailsManager,
}

pub struct ContextBuilder {
	port: Option<u16>,
	auth_secret: Vec<u8>,
	web_dir_path: Option<PathBuf>,
	swagger_dir_path: Option<PathBuf>,
	cache_dir_path: Option<PathBuf>,
	index: Index,
	db: DB,
}

impl ContextBuilder {
	pub fn new(db: DB, index: Index) -> Self {
		Self {
			port: None,
			auth_secret: [0; 32].into(),
			web_dir_path: None,
			swagger_dir_path: None,
			cache_dir_path: None,
			index,
			db,
		}
	}

	pub fn build(self) -> anyhow::Result<Context> {
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

		Ok(Context {
			port: self.port.unwrap_or(5050),
			auth_secret: self.auth_secret,
			api_url: "/api".to_owned(),
			swagger_url: "/swagger".to_owned(),
			web_url: "/".to_owned(),
			web_dir_path,
			swagger_dir_path,
			thumbnails_manager: ThumbnailsManager::new(thumbnails_dir_path),
			index: self.index,
			db: self.db,
		})
	}

	pub fn port(mut self, port: u16) -> Self {
		self.port = Some(port);
		self
	}

	pub fn auth_secret(mut self, secret: Vec<u8>) -> Self {
		self.auth_secret = secret;
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
