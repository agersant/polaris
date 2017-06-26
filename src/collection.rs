use core::ops::Deref;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use config::Config;
use errors::*;
use db::*;
use vfs::*;


#[derive(Clone, Debug, Deserialize)]
pub struct User {
	name: String,
	password: String,
}

pub struct Collection {
	vfs: Arc<Vfs>,
	users: Vec<User>,
	db: Arc<DB>,
}

impl Collection {
	pub fn new(vfs: Arc<Vfs>, db: Arc<DB>) -> Collection {
		Collection {
			vfs: vfs,
			users: Vec::new(),
			db: db,
		}
	}

	pub fn load_config(&mut self, config: &Config) -> Result<()> {
		self.users = config.users.to_vec();
		Ok(())
	}

	pub fn auth(&self, username: &str, password: &str) -> bool {
		self.users
			.iter()
			.any(|u| u.name == username && u.password == password)
	}

	pub fn browse(&self, virtual_path: &Path) -> Result<Vec<CollectionFile>> {
		self.db.deref().browse(virtual_path)
	}

	pub fn flatten(&self, virtual_path: &Path) -> Result<Vec<Song>> {
		self.db.deref().flatten(virtual_path)
	}

	pub fn get_random_albums(&self, count: i64) -> Result<Vec<Directory>> {
		self.db.deref().get_random_albums(count)
	}

	pub fn get_recent_albums(&self, count: i64) -> Result<Vec<Directory>> {
		self.db.deref().get_recent_albums(count)
	}

	pub fn locate(&self, virtual_path: &Path) -> Result<PathBuf> {
		self.vfs.virtual_to_real(virtual_path)
	}
}
