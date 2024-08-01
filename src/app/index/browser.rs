use std::{
	collections::{HashMap, HashSet},
	hash::Hash,
	path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use trie_rs::{Trie, TrieBuilder};

use crate::app::{index::Error, scanner};

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum File {
	Directory(PathBuf),
	Song(PathBuf),
}

#[derive(Serialize, Deserialize)]
pub struct Browser {
	directories: HashMap<PathBuf, HashSet<File>>,
	flattened: Trie<String>,
}

impl Browser {
	pub fn new() -> Self {
		Self {
			directories: HashMap::new(),
			flattened: TrieBuilder::new().build(),
		}
	}

	pub fn browse<P: AsRef<Path>>(&self, virtual_path: P) -> Result<Vec<File>, Error> {
		let Some(files) = self.directories.get(virtual_path.as_ref()) else {
			return Err(Error::DirectoryNotFound(virtual_path.as_ref().to_owned()));
		};
		Ok(files.iter().cloned().collect())
	}

	pub fn flatten<P: AsRef<Path>>(&self, virtual_path: P) -> Result<Vec<PathBuf>, Error> {
		let path_components = virtual_path
			.as_ref()
			.components()
			.map(|c| c.as_os_str().to_string_lossy().to_string())
			.collect::<Vec<String>>();

		if !self.flattened.is_prefix(&path_components) {
			return Err(Error::DirectoryNotFound(virtual_path.as_ref().to_owned()));
		}

		Ok(self
			.flattened
			.predictive_search(path_components)
			.map(|c: Vec<String>| -> PathBuf { c.join(std::path::MAIN_SEPARATOR_STR).into() })
			.collect::<Vec<_>>())
	}
}

pub struct Builder {
	directories: HashMap<PathBuf, HashSet<File>>,
	flattened: TrieBuilder<String>,
}

impl Default for Builder {
	fn default() -> Self {
		Self {
			directories: Default::default(),
			flattened: Default::default(),
		}
	}
}

impl Builder {
	pub fn add_directory(&mut self, directory: scanner::Directory) {
		self.directories
			.entry(directory.virtual_path.clone())
			.or_default();

		if let Some(parent) = directory.virtual_parent {
			self.directories
				.entry(parent.clone())
				.or_default()
				.insert(File::Directory(directory.virtual_path));
		}
	}

	pub fn add_song(&mut self, song: &scanner::Song) {
		self.directories
			.entry(song.virtual_parent.clone())
			.or_default()
			.insert(File::Song(song.virtual_path.clone()));

		self.flattened.push(
			song.virtual_path
				.components()
				.map(|c| c.as_os_str().to_string_lossy().to_string())
				.collect::<Vec<_>>(),
		);
	}

	pub fn build(self) -> Browser {
		Browser {
			directories: self.directories,
			flattened: self.flattened.build(),
		}
	}
}

#[cfg(test)]
mod test {
	use std::path::{Path, PathBuf};

	use super::*;
	use crate::app::test;
	use crate::test_name;

	const TEST_MOUNT_NAME: &str = "root";

	#[tokio::test]
	async fn can_browse_top_level() {
		let mut ctx = test::ContextBuilder::new(test_name!())
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build()
			.await;
		ctx.scanner.update().await.unwrap();

		let root_path = Path::new(TEST_MOUNT_NAME);
		let files = ctx.index_manager.browse(PathBuf::new()).await.unwrap();
		assert_eq!(files.len(), 1);
		match files[0] {
			File::Directory(ref d) => {
				assert_eq!(d, &root_path)
			}
			_ => panic!("Expected directory"),
		}
	}

	#[tokio::test]
	async fn can_browse_directory() {
		let khemmis_path: PathBuf = [TEST_MOUNT_NAME, "Khemmis"].iter().collect();
		let tobokegao_path: PathBuf = [TEST_MOUNT_NAME, "Tobokegao"].iter().collect();

		let mut ctx = test::ContextBuilder::new(test_name!())
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build()
			.await;
		ctx.scanner.update().await.unwrap();

		let files = ctx
			.index_manager
			.browse(PathBuf::from(TEST_MOUNT_NAME))
			.await
			.unwrap();

		assert_eq!(files.len(), 2);
		match files[0] {
			File::Directory(ref d) => {
				assert_eq!(d, &khemmis_path)
			}
			_ => panic!("Expected directory"),
		}

		match files[1] {
			File::Directory(ref d) => {
				assert_eq!(d, &tobokegao_path)
			}
			_ => panic!("Expected directory"),
		}
	}

	#[tokio::test]
	async fn can_flatten_root() {
		let mut ctx = test::ContextBuilder::new(test_name!())
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build()
			.await;
		ctx.scanner.update().await.unwrap();
		let songs = ctx
			.index_manager
			.flatten(PathBuf::from(TEST_MOUNT_NAME))
			.await
			.unwrap();
		assert_eq!(songs.len(), 13);
		assert_eq!(songs[0], Path::new("FIX ME"));
	}

	#[tokio::test]
	async fn can_flatten_directory() {
		let mut ctx = test::ContextBuilder::new(test_name!())
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build()
			.await;
		ctx.scanner.update().await.unwrap();
		let path: PathBuf = [TEST_MOUNT_NAME, "Tobokegao"].iter().collect();
		let songs = ctx.index_manager.flatten(path).await.unwrap();
		assert_eq!(songs.len(), 8);
	}

	#[tokio::test]
	async fn can_flatten_directory_with_shared_prefix() {
		let mut ctx = test::ContextBuilder::new(test_name!())
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build()
			.await;
		ctx.scanner.update().await.unwrap();
		let path: PathBuf = [TEST_MOUNT_NAME, "Tobokegao", "Picnic"].iter().collect(); // Prefix of '(Picnic Remixes)'
		let songs = ctx.index_manager.flatten(path).await.unwrap();
		assert_eq!(songs.len(), 7);
	}
}
