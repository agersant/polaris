use std::path::PathBuf;

use crate::app::{config, ddns, index, playlist, scanner, settings, user, vfs};
use crate::db::DB;
use crate::test::*;

pub struct Context {
	pub index_manager: index::Manager,
	pub scanner: scanner::Scanner,
	pub config_manager: config::Manager,
	pub ddns_manager: ddns::Manager,
	pub playlist_manager: playlist::Manager,
	pub settings_manager: settings::Manager,
	pub user_manager: user::Manager,
	pub vfs_manager: vfs::Manager,
}

pub struct ContextBuilder {
	config: config::Config,
	pub test_directory: PathBuf,
}

impl ContextBuilder {
	pub fn new(test_name: String) -> Self {
		Self {
			test_directory: prepare_test_directory(test_name),
			config: config::Config::default(),
		}
	}

	pub fn user(mut self, name: &str, password: &str, is_admin: bool) -> Self {
		self.config
			.users
			.get_or_insert(Vec::new())
			.push(user::NewUser {
				name: name.to_owned(),
				password: password.to_owned(),
				admin: is_admin,
			});
		self
	}

	pub fn mount(mut self, name: &str, source: &str) -> Self {
		self.config
			.mount_dirs
			.get_or_insert(Vec::new())
			.push(vfs::MountDir {
				name: name.to_owned(),
				source: source.to_owned(),
			});
		self
	}
	pub async fn build(self) -> Context {
		let db_path = self.test_directory.join("db.sqlite");

		let db = DB::new(&db_path).await.unwrap();
		let settings_manager = settings::Manager::new(db.clone());
		let auth_secret = settings_manager.get_auth_secret().await.unwrap();
		let user_manager = user::Manager::new(db.clone(), auth_secret);
		let vfs_manager = vfs::Manager::new(db.clone());
		let ddns_manager = ddns::Manager::new(db.clone());
		let config_manager = config::Manager::new(
			settings_manager.clone(),
			user_manager.clone(),
			vfs_manager.clone(),
			ddns_manager.clone(),
		);
		let index_manager = index::Manager::new(db.clone()).await;
		let scanner = scanner::Scanner::new(
			index_manager.clone(),
			settings_manager.clone(),
			vfs_manager.clone(),
		)
		.await
		.unwrap();
		let playlist_manager = playlist::Manager::new(db.clone());

		config_manager.apply(&self.config).await.unwrap();

		Context {
			index_manager,
			scanner,
			config_manager,
			ddns_manager,
			playlist_manager,
			settings_manager,
			user_manager,
			vfs_manager,
		}
	}
}
