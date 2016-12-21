use core::ops::Deref;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use config::Config;
use errors::*;
use index::*;
use vfs::*;


#[derive(Clone, Debug)]
pub struct User {
	name: String,
	password: String,
}

impl User {
	pub fn new(name: String, password: String) -> User {
		User {
			name: name,
			password: password,
		}
	}
}

pub struct Collection {
	vfs: Arc<Vfs>,
	users: Vec<User>,
	index: Arc<Index>,
}

impl Collection {
	pub fn new(vfs: Arc<Vfs>, index: Arc<Index>) -> Collection {
		Collection {
			vfs: vfs,
			users: Vec::new(),
			index: index,
		}
	}

	pub fn load_config(&mut self, config: &Config) -> Result<()> {
		self.users = config.users.to_vec();
		Ok(())
	}

	pub fn auth(&self, username: &str, password: &str) -> bool {
		self.users.iter().any(|u| u.name == username && u.password == password)
	}

	pub fn browse(&self, virtual_path: &Path) -> Result<Vec<CollectionFile>> {
		self.index.deref().browse(virtual_path)
	}

	pub fn flatten(&self, virtual_path: &Path) -> Result<Vec<Song>> {
		self.index.deref().flatten(virtual_path)
	}

	pub fn get_random_albums(&self, count: u32) -> Result<Vec<Directory>> {
		self.index.deref().get_random_albums(count)
	}

	pub fn locate(&self, virtual_path: &Path) -> Result<PathBuf> {
		self.vfs.virtual_to_real(virtual_path)
	}
}
