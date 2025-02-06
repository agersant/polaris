use std::{
	cmp::Ordering,
	collections::{BTreeSet, HashMap},
	ffi::OsStr,
	hash::Hash,
	path::{Path, PathBuf},
};

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use tinyvec::TinyVec;
use trie_rs::{Trie, TrieBuilder};

use crate::app::index::{
	dictionary::{self, Dictionary},
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
		dictionary: &Dictionary,
		virtual_path: P,
	) -> Result<Vec<File>, Error> {
		let path = virtual_path
			.as_ref()
			.get(dictionary)
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
				let path = Path::new(OsStr::new(dictionary.resolve(&path.0))).to_owned();
				match f {
					storage::File::Directory(_) => File::Directory(path),
					storage::File::Song(_) => File::Song(path),
				}
			})
			.collect::<Vec<_>>();

		if virtual_path.as_ref().parent().is_none() {
			if let [File::Directory(ref p)] = files[..] {
				return self.browse(dictionary, p);
			}
		}

		let collator = dictionary::make_collator();
		files.sort_by(|a, b| {
			let (a, b) = match (a, b) {
				(File::Directory(_), File::Song(_)) => return Ordering::Less,
				(File::Song(_), File::Directory(_)) => return Ordering::Greater,
				(File::Directory(a), File::Directory(b)) => (a, b),
				(File::Song(a), File::Song(b)) => (a, b),
			};
			collator.compare(
				a.as_os_str().to_string_lossy().as_ref(),
				b.as_os_str().to_string_lossy().as_ref(),
			)
		});

		Ok(files)
	}

	pub fn flatten<P: AsRef<Path>>(
		&self,
		dictionary: &Dictionary,
		virtual_path: P,
	) -> Result<Vec<PathBuf>, Error> {
		let path_components = virtual_path
			.as_ref()
			.components()
			.map(|c| c.as_os_str().to_str().unwrap_or_default())
			.filter_map(|c| dictionary.get(c))
			.collect::<Vec<_>>();

		if !self.flattened.is_prefix(&path_components) {
			return Err(Error::DirectoryNotFound(virtual_path.as_ref().to_owned()));
		}

		let mut results: Vec<TinyVec<[_; 8]>> = self
			.flattened
			.predictive_search(path_components)
			.collect::<Vec<_>>();

		results.par_sort_unstable_by(|a, b| {
			for (x, y) in a.iter().zip(b.iter()) {
				match dictionary.cmp(x, y) {
					Ordering::Equal => continue,
					ordering => return ordering,
				}
			}
			a.len().cmp(&b.len())
		});

		let files = results
			.into_iter()
			.map(|c: TinyVec<[_; 8]>| -> PathBuf {
				c.into_iter()
					.map(|s| dictionary.resolve(&s))
					.collect::<TinyVec<[&str; 8]>>()
					.join(std::path::MAIN_SEPARATOR_STR)
					.into()
			})
			.collect::<Vec<_>>();

		Ok(files)
	}
}

#[derive(Clone, Default)]
pub struct Builder {
	directories: HashMap<PathKey, BTreeSet<storage::File>>,
	flattened: TrieBuilder<lasso2::Spur>,
}

impl Builder {
	pub fn add_directory(
		&mut self,
		dictionary_builder: &mut dictionary::Builder,
		directory: scanner::Directory,
	) {
		let Some(virtual_path) = (&directory.virtual_path).get_or_intern(dictionary_builder) else {
			return;
		};

		let Some(virtual_parent) = directory
			.virtual_path
			.parent()
			.and_then(|p| p.get_or_intern(dictionary_builder))
		else {
			return;
		};

		self.directories.entry(virtual_path).or_default();

		self.directories
			.entry(virtual_parent)
			.or_default()
			.insert(storage::File::Directory(virtual_path));
	}

	pub fn add_song(&mut self, dictionary_builder: &mut dictionary::Builder, song: &scanner::Song) {
		let Some(virtual_path) = (&song.virtual_path).get_or_intern(dictionary_builder) else {
			return;
		};

		let Some(virtual_parent) = song
			.virtual_path
			.parent()
			.and_then(|p| p.get_or_intern(dictionary_builder))
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
				.map(|c| dictionary_builder.get_or_intern(c.as_os_str().to_str().unwrap()))
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

	fn setup_test(songs: HashSet<PathBuf>) -> (Browser, Dictionary) {
		let mut dictionary_builder = dictionary::Builder::default();
		let mut builder = Builder::default();

		let directories = songs
			.iter()
			.flat_map(|k| k.parent().unwrap().ancestors())
			.collect::<HashSet<_>>();

		for directory in directories {
			builder.add_directory(
				&mut dictionary_builder,
				scanner::Directory {
					virtual_path: directory.to_owned(),
				},
			);
		}

		for path in songs {
			let mut song = scanner::Song::default();
			song.virtual_path = path.clone();
			builder.add_song(&mut dictionary_builder, &song);
		}

		let browser = builder.build();
		let dictionary = dictionary_builder.build();

		(browser, dictionary)
	}

	#[test]
	fn can_browse_top_level() {
		let (browser, strings) = setup_test(HashSet::from([
			PathBuf::from_iter(["Music", "Iron Maiden", "Moonchild.mp3"]),
			PathBuf::from_iter(["Also Music", "Iron Maiden", "The Prisoner.mp3"]),
		]));
		let files = browser.browse(&strings, PathBuf::new()).unwrap();
		assert_eq!(
			files[..],
			[
				File::Directory(PathBuf::from_iter(["Also Music"])),
				File::Directory(PathBuf::from_iter(["Music"])),
			]
		);
	}

	#[test]
	fn browse_skips_redundant_top_level() {
		let (browser, strings) = setup_test(HashSet::from([PathBuf::from_iter([
			"Music",
			"Iron Maiden",
			"Moonchild.mp3",
		])]));
		let files = browser.browse(&strings, PathBuf::new()).unwrap();
		assert_eq!(
			files[..],
			[File::Directory(PathBuf::from_iter([
				"Music",
				"Iron Maiden"
			])),]
		);
	}

	#[test]
	fn can_browse_directory() {
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

	#[test]
	fn browse_entries_are_sorted() {
		let (browser, strings) = setup_test(HashSet::from([
			PathBuf::from_iter(["Ott", "Mir.mp3"]),
			PathBuf::from("Helios.mp3"),
			PathBuf::from("asura.mp3"),
			PathBuf::from("à la maison.mp3"),
		]));

		let files = browser.browse(&strings, PathBuf::new()).unwrap();

		assert_eq!(
			files,
			[
				File::Directory(PathBuf::from("Ott")),
				File::Song(PathBuf::from("à la maison.mp3")),
				File::Song(PathBuf::from("asura.mp3")),
				File::Song(PathBuf::from("Helios.mp3")),
			]
		);
	}

	#[test]
	fn can_flatten_root() {
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

	#[test]
	fn can_flatten_directory() {
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

	#[test]
	fn flatten_entries_are_sorted() {
		let (browser, strings) = setup_test(HashSet::from([
			PathBuf::from_iter(["Ott", "Mir.mp3"]),
			PathBuf::from("Helios.mp3"),
			PathBuf::from("à la maison.mp3.mp3"),
			PathBuf::from("asura.mp3"),
		]));

		let files = browser.flatten(&strings, PathBuf::new()).unwrap();

		assert_eq!(
			files,
			[
				PathBuf::from("à la maison.mp3.mp3"),
				PathBuf::from("asura.mp3"),
				PathBuf::from("Helios.mp3"),
				PathBuf::from_iter(["Ott", "Mir.mp3"]),
			]
		);
	}

	#[test]
	fn can_flatten_directory_with_shared_prefix() {
		let directory_a = PathBuf::from_iter(["Music", "Therion", "Leviathan II"]);
		let directory_b = PathBuf::from_iter(["Music", "Therion", "Leviathan III"]);
		let song_a = directory_a.join("Pazuzu.mp3");
		let song_b = directory_b.join("Ninkigal.mp3");

		let (browser, strings) = setup_test(HashSet::from([song_a.clone(), song_b.clone()]));

		let files = browser.flatten(&strings, directory_a).unwrap();

		assert_eq!(files, [song_a]);
	}
}
