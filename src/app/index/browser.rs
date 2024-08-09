use std::{
	collections::{BTreeSet, HashMap},
	ffi::OsStr,
	hash::Hash,
	path::{Path, PathBuf},
};

use lasso2::{RodeoReader, ThreadedRodeo};
use serde::{Deserialize, Serialize};
use tinyvec::TinyVec;
use trie_rs::{Trie, TrieBuilder};

use crate::app::index::{
	storage::{self, PathKey},
	InternPath,
};
use crate::app::{scanner, Error};

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum File {
	Directory(PathBuf),
	Song(PathBuf),
}

#[derive(Serialize, Deserialize)]
pub struct Browser {
	directories: HashMap<PathKey, BTreeSet<storage::File>>,
	flattened: Trie<lasso2::Spur>,
}

impl Default for Browser {
	fn default() -> Self {
		Self {
			directories: HashMap::default(),
			flattened: TrieBuilder::new().build(),
		}
	}
}

impl Browser {
	pub fn browse<P: AsRef<Path>>(
		&self,
		strings: &RodeoReader,
		virtual_path: P,
	) -> Result<Vec<File>, Error> {
		let path = virtual_path
			.as_ref()
			.get(strings)
			.ok_or_else(|| Error::DirectoryNotFound(virtual_path.as_ref().to_owned()))?;

		let Some(files) = self.directories.get(&path) else {
			return Err(Error::DirectoryNotFound(virtual_path.as_ref().to_owned()));
		};

		Ok(files
			.iter()
			.map(|f| {
				let path = match f {
					storage::File::Directory(p) => p,
					storage::File::Song(p) => p,
				};
				let path = Path::new(OsStr::new(strings.resolve(&path.0))).to_owned();
				match f {
					storage::File::Directory(_) => File::Directory(path),
					storage::File::Song(_) => File::Song(path),
				}
			})
			.collect())
	}

	pub fn flatten<P: AsRef<Path>>(
		&self,
		strings: &RodeoReader,
		virtual_path: P,
	) -> Result<Vec<PathBuf>, Error> {
		let path_components = virtual_path
			.as_ref()
			.components()
			.map(|c| c.as_os_str().to_str().unwrap_or_default())
			.filter_map(|c| strings.get(c))
			.collect::<Vec<_>>();

		if !self.flattened.is_prefix(&path_components) {
			return Err(Error::DirectoryNotFound(virtual_path.as_ref().to_owned()));
		}

		Ok(self
			.flattened
			.predictive_search(path_components)
			.map(|c: TinyVec<[_; 8]>| -> PathBuf {
				c.into_iter()
					.map(|s| strings.resolve(&s))
					.collect::<TinyVec<[&str; 8]>>()
					.join(std::path::MAIN_SEPARATOR_STR)
					.into()
			})
			.collect::<Vec<_>>())
	}
}

#[derive(Default)]
pub struct Builder {
	directories: HashMap<PathKey, BTreeSet<storage::File>>,
	flattened: TrieBuilder<lasso2::Spur>,
}

impl Builder {
	pub fn add_directory(&mut self, strings: &mut ThreadedRodeo, directory: scanner::Directory) {
		let Some(virtual_path) = directory.virtual_path.get_or_intern(strings) else {
			return;
		};

		let Some(virtual_parent) = directory
			.virtual_parent
			.and_then(|p| p.get_or_intern(strings))
		else {
			return;
		};

		self.directories.entry(virtual_path).or_default();

		self.directories
			.entry(virtual_parent)
			.or_default()
			.insert(storage::File::Directory(virtual_path));
	}

	pub fn add_song(&mut self, strings: &mut ThreadedRodeo, song: &scanner::Song) {
		let Some(virtual_path) = (&song.virtual_path).get_or_intern(strings) else {
			return;
		};

		let Some(virtual_parent) = (&song.virtual_parent).get_or_intern(strings) else {
			return;
		};

		self.directories
			.entry(virtual_parent)
			.or_default()
			.insert(storage::File::Song(virtual_path));

		self.flattened.push(
			song.virtual_path
				.components()
				.map(|c| strings.get_or_intern(c.as_os_str().to_str().unwrap()))
				.collect::<TinyVec<[lasso2::Spur; 8]>>(),
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
