use crate::db::DB;
use crate::index::Index;
use crate::thumbnails::ThumbnailsManager;
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
	context: Context,
}

impl ContextBuilder {
	pub fn new(db: DB, index: Index, thumbnails_manager: ThumbnailsManager) -> Self {
		Self {
			context: Context {
				port: 5050,
				auth_secret: [0; 32].into(),
				api_url: "/api".to_owned(),
				swagger_url: "/swagger".to_owned(),
				web_url: "/".to_owned(),
				web_dir_path: PathBuf::new(),
				swagger_dir_path: PathBuf::new(),
				db,
				index,
				thumbnails_manager,
			},
		}
	}

	pub fn build(self) -> Context {
		self.context
	}

	pub fn port(mut self, port: u16) -> Self {
		self.context.port = port;
		self
	}

	pub fn auth_secret(mut self, secret: Vec<u8>) -> Self {
		self.context.auth_secret = secret;
		self
	}

	pub fn web_dir_path(mut self, path: PathBuf) -> Self {
		self.context.web_dir_path = path;
		self
	}

	pub fn swagger_dir_path(mut self, path: PathBuf) -> Self {
		self.context.swagger_dir_path = path;
		self
	}
}
