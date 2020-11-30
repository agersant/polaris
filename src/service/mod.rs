use crate::db::DB;
use crate::index::Index;
use crate::thumbnails::ThumbnailsManager;
use std::path::{Path, PathBuf};

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
	port: u16,
	auth_secret: Vec<u8>,
	web_dir_path: PathBuf,
	swagger_dir_path: PathBuf,
	thumbnails_dir_path: PathBuf,
	web_url: String,
	swagger_url: String,
	api_url: String,
	index: Index,
	db: DB,
}

impl ContextBuilder {
	pub fn new(db: DB, index: Index) -> Self {
		Self {
			port: 5050,
			auth_secret: [0; 32].into(),
			api_url: "/api".to_owned(),
			swagger_url: "/swagger".to_owned(),
			web_url: "/".to_owned(),
			web_dir_path: Path::new("web").into(),
			swagger_dir_path: Path::new("swagger").into(),
			thumbnails_dir_path: Path::new("thumbnails").into(),
			index,
			db,
		}
	}

	pub fn build(self) -> Context {
		Context {
			port: self.port,
			auth_secret: self.auth_secret,
			api_url: self.api_url,
			swagger_url: self.swagger_url,
			web_url: self.web_url,
			web_dir_path: self.web_dir_path,
			swagger_dir_path: self.swagger_dir_path,
			thumbnails_manager: ThumbnailsManager::new(self.thumbnails_dir_path),
			index: self.index,
			db: self.db,
		}
	}

	pub fn port(mut self, port: u16) -> Self {
		self.port = port;
		self
	}

	pub fn auth_secret(mut self, secret: Vec<u8>) -> Self {
		self.auth_secret = secret;
		self
	}

	pub fn web_dir_path(mut self, path: PathBuf) -> Self {
		self.web_dir_path = path;
		self
	}

	pub fn swagger_dir_path(mut self, path: PathBuf) -> Self {
		self.swagger_dir_path = path;
		self
	}

	pub fn thumbnails_dir_path(mut self, path: PathBuf) -> Self {
		self.thumbnails_dir_path = path;
		self
	}
}
