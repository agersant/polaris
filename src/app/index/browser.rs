use std::{
	collections::{BTreeSet, HashMap},
	ffi::OsStr,
	hash::Hash,
	path::{Path, PathBuf},
};

use lasso2::{Rodeo, RodeoReader};
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

		let mut files = files
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
			.collect::<Vec<_>>();

		files.sort();

		Ok(files)
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

		let mut files = self
			.flattened
			.predictive_search(path_components)
			.map(|c: TinyVec<[_; 8]>| -> PathBuf {
				c.into_iter()
					.map(|s| strings.resolve(&s))
					.collect::<TinyVec<[&str; 8]>>()
					.join(std::path::MAIN_SEPARATOR_STR)
					.into()
			})
			.collect::<Vec<_>>();

		files.sort();

		Ok(files)
	}
}

#[derive(Default)]
pub struct Builder {
	directories: HashMap<PathKey, BTreeSet<storage::File>>,
	flattened: TrieBuilder<lasso2::Spur>,
}

impl Builder {
	pub fn add_directory(&mut self, strings: &mut Rodeo, directory: scanner::Directory) {
		let Some(virtual_path) = (&directory.virtual_path).get_or_intern(strings) else {
			return;
		};

		let Some(virtual_parent) = directory
			.virtual_path
			.parent()
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

	pub fn add_song(&mut self, strings: &mut Rodeo, song: &scanner::Song) {
		let Some(virtual_path) = (&song.virtual_path).get_or_intern(strings) else {
			return;
		};

		let Some(virtual_parent) = song
			.virtual_path
			.parent()
			.and_then(|p| p.get_or_intern(strings))
		else {
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
	use std::collections::HashSet;
	use std::path::PathBuf;

	use super::*;

	fn setup_test(songs: HashSet<PathBuf>) -> (Browser, RodeoReader) {
		let mut strings = Rodeo::new();
		let mut builder = Builder::default();

		let directories = songs
			.iter()
			.flat_map(|k| k.parent().unwrap().ancestors())
			.collect::<HashSet<_>>();

		for directory in directories {
			builder.add_directory(
				&mut strings,
				scanner::Directory {
					virtual_path: directory.to_owned(),
				},
			);
		}

		for path in songs {
			let mut song = scanner::Song::default();
			song.virtual_path = path.clone();
			builder.add_song(&mut strings, &song);
		}

		let browser = builder.build();
		let strings = strings.into_reader();

		(browser, strings)
	}

	#[tokio::test]
	async fn can_browse_top_level() {
		let song_a = PathBuf::from_iter(["Music", "Iron Maiden", "Moonchild.mp3"]);
		let (browser, strings) = setup_test(HashSet::from([song_a]));
		let files = browser.browse(&strings, PathBuf::new()).unwrap();
		assert_eq!(files.len(), 1);
		assert_eq!(files[0], File::Directory(PathBuf::from_iter(["Music"])));
	}

	#[tokio::test]
	async fn can_browse_directory() {
		let artist_directory = PathBuf::from_iter(["Music", "Iron Maiden"]);

		let (browser, strings) = setup_test(HashSet::from([
			artist_directory.join("Infinite Dreams.mp3"),
			artist_directory.join("Moonchild.mp3"),
		]));

		let files = browser.browse(&strings, artist_directory.clone()).unwrap();

		assert_eq!(
			files,
			[
				File::Song(artist_directory.join("Infinite Dreams.mp3")),
				File::Song(artist_directory.join("Moonchild.mp3"))
			]
		);
	}

	#[tokio::test]
	async fn can_flatten_root() {
		let song_a = PathBuf::from_iter(["Music", "Electronic", "Papua New Guinea.mp3"]);
		let song_b = PathBuf::from_iter(["Music", "Metal", "Destiny.mp3"]);
		let song_c = PathBuf::from_iter(["Music", "Metal", "No Turning Back.mp3"]);

		let (browser, strings) = setup_test(HashSet::from([
			song_a.clone(),
			song_b.clone(),
			song_c.clone(),
		]));

		let files = browser.flatten(&strings, PathBuf::new()).unwrap();

		assert_eq!(files, [song_a, song_b, song_c]);
	}

	#[tokio::test]
	async fn can_flatten_directory() {
		let electronic = PathBuf::from_iter(["Music", "Electronic"]);
		let song_a = electronic.join(PathBuf::from_iter(["FSOL", "Papua New Guinea.mp3"]));
		let song_b = electronic.join(PathBuf::from_iter(["Kraftwerk", "Autobahn.mp3"]));
		let song_c = PathBuf::from_iter(["Music", "Metal", "Destiny.mp3"]);

		let (browser, strings) = setup_test(HashSet::from([
			song_a.clone(),
			song_b.clone(),
			song_c.clone(),
		]));

		let files = browser.flatten(&strings, electronic).unwrap();

		assert_eq!(files, [song_a, song_b]);
	}

	#[tokio::test]
	async fn can_flatten_directory_with_shared_prefix() {
		let directory_a = PathBuf::from_iter(["Music", "Therion", "Leviathan II"]);
		let directory_b = PathBuf::from_iter(["Music", "Therion", "Leviathan III"]);
		let song_a = directory_a.join("Pazuzu.mp3");
		let song_b = directory_b.join("Ninkigal.mp3");

		let (browser, strings) = setup_test(HashSet::from([song_a.clone(), song_b.clone()]));

		let files = browser.flatten(&strings, directory_a).unwrap();

		assert_eq!(files, [song_a]);
	}
}
