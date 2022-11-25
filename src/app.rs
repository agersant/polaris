use std::fs;
use std::path::PathBuf;

use crate::db::{self, DB};
use crate::paths::Paths;

pub mod config;
pub mod ddns;
pub mod index;
pub mod lastfm;
pub mod playlist;
pub mod settings;
pub mod thumbnail;
pub mod user;
pub mod vfs;

#[cfg(test)]
pub mod test;

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error(transparent)]
	Config(#[from] config::Error),
	#[error(transparent)]
	Database(#[from] db::Error),
	#[error("Filesystem error for `{0}`: `{1}`")]
	Io(PathBuf, std::io::Error),
	#[error(transparent)]
	Settings(#[from] settings::Error),
}

#[derive(Clone)]
pub struct App {
	pub port: u16,
	pub auth_secret: settings::AuthSecret,
	pub web_dir_path: PathBuf,
	pub swagger_dir_path: PathBuf,
	pub db: DB,
	pub index: index::Index,
	pub config_manager: config::Manager,
	pub ddns_manager: ddns::Manager,
	pub lastfm_manager: lastfm::Manager,
	pub playlist_manager: playlist::Manager,
	pub settings_manager: settings::Manager,
	pub thumbnail_manager: thumbnail::Manager,
	pub user_manager: user::Manager,
	pub vfs_manager: vfs::Manager,
}

impl App {
	pub fn new(port: u16, paths: Paths) -> Result<Self, Error> {
		let db = DB::new(&paths.db_file_path)?;
		fs::create_dir_all(&paths.web_dir_path)
			.map_err(|e| Error::Io(paths.web_dir_path.clone(), e))?;
		fs::create_dir_all(&paths.swagger_dir_path)
			.map_err(|e| Error::Io(paths.swagger_dir_path.clone(), e))?;

		let thumbnails_dir_path = paths.cache_dir_path.join("thumbnails");
		fs::create_dir_all(&thumbnails_dir_path)
			.map_err(|e| Error::Io(thumbnails_dir_path.clone(), e))?;

		let vfs_manager = vfs::Manager::new(db.clone());
		let settings_manager = settings::Manager::new(db.clone());
		let auth_secret = settings_manager.get_auth_secret()?;
		let ddns_manager = ddns::Manager::new(db.clone());
		let user_manager = user::Manager::new(db.clone(), auth_secret);
		let index = index::Index::new(db.clone(), vfs_manager.clone(), settings_manager.clone());
		let config_manager = config::Manager::new(
			settings_manager.clone(),
			user_manager.clone(),
			vfs_manager.clone(),
			ddns_manager.clone(),
		);
		let playlist_manager = playlist::Manager::new(db.clone(), vfs_manager.clone());
		let thumbnail_manager = thumbnail::Manager::new(thumbnails_dir_path);
		let lastfm_manager = lastfm::Manager::new(index.clone(), user_manager.clone());

		if let Some(config_path) = paths.config_file_path {
			let config = config::Config::from_path(&config_path)?;
			config_manager.apply(&config)?;
		}

		let auth_secret = settings_manager.get_auth_secret()?;

		Ok(Self {
			port,
			auth_secret,
			web_dir_path: paths.web_dir_path,
			swagger_dir_path: paths.swagger_dir_path,
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
}
