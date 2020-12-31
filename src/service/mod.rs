use std::fs;
use std::path::PathBuf;

use crate::app::{config, ddns, index::Index, lastfm, playlist, settings, thumbnail, user, vfs};
use crate::db::DB;
use crate::paths::Paths;

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

pub struct ContextBuilder {
	port: Option<u16>,
	paths: Paths,
}

impl ContextBuilder {
	pub fn new(paths: Paths) -> Self {
		Self { port: None, paths }
	}

	pub fn build(self) -> anyhow::Result<Context> {
		let db = DB::new(&self.paths.db_file_path)?;
		fs::create_dir_all(&self.paths.web_dir_path)?;
		fs::create_dir_all(&self.paths.swagger_dir_path)?;

		let thumbnails_dir_path = self.paths.cache_dir_path.join("thumbnails");

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

		if let Some(config_path) = self.paths.config_file_path {
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
			web_dir_path: self.paths.web_dir_path,
			swagger_dir_path: self.paths.swagger_dir_path,
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
}
