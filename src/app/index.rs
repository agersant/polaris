use std::{
	path::{Path, PathBuf},
	sync::{Arc, RwLock},
};

use log::{error, info};
use serde::{Deserialize, Serialize};
use tokio::task::spawn_blocking;

use crate::app::{scanner, Error};

mod browser;
mod collection;
mod dictionary;
mod query;
mod search;
mod storage;

pub use browser::File;
pub use collection::{Album, AlbumHeader, Artist, ArtistHeader, Genre, GenreHeader, Song};
use storage::{store_song, AlbumKey, ArtistKey, GenreKey, InternPath, SongKey};

#[derive(Clone)]
pub struct Manager {
	index_file_path: PathBuf,
	index: Arc<RwLock<Index>>, // Not a tokio RwLock as we want to do CPU-bound work with Index and lock this inside spawn_blocking()
}

impl Manager {
	pub async fn new(directory: &Path) -> Result<Self, Error> {
		tokio::fs::create_dir_all(directory)
			.await
			.map_err(|e| Error::Io(directory.to_owned(), e))?;

		let index_manager = Self {
			index_file_path: directory.join("collection.index"),
			index: Arc::default(),
		};

		match index_manager.try_restore_index().await {
			Ok(true) => info!("Restored collection index from disk"),
			Ok(false) => info!("No existing collection index to restore"),
			Err(e) => error!("Failed to restore collection index: {}", e),
		};

		Ok(index_manager)
	}

	pub async fn is_index_empty(&self) -> bool {
		spawn_blocking({
			let index_manager = self.clone();
			move || {
				let index = index_manager.index.read().unwrap();
				index.collection.num_songs() == 0
			}
		})
		.await
		.unwrap()
	}

	pub async fn replace_index(&self, new_index: Index) {
		spawn_blocking({
			let index_manager = self.clone();
			move || {
				let mut lock = index_manager.index.write().unwrap();
				*lock = new_index;
			}
		})
		.await
		.unwrap()
	}

	pub async fn persist_index(&self, index: &Index) -> Result<(), Error> {
		let serialized = match bitcode::serialize(index) {
			Ok(s) => s,
			Err(_) => return Err(Error::IndexSerializationError),
		};
		tokio::fs::write(&self.index_file_path, &serialized[..])
			.await
			.map_err(|e| Error::Io(self.index_file_path.clone(), e))?;
		Ok(())
	}

	async fn try_restore_index(&self) -> Result<bool, Error> {
		match tokio::fs::try_exists(&self.index_file_path).await {
			Ok(true) => (),
			Ok(false) => return Ok(false),
			Err(e) => return Err(Error::Io(self.index_file_path.clone(), e)),
		};

		let serialized = tokio::fs::read(&self.index_file_path)
			.await
			.map_err(|e| Error::Io(self.index_file_path.clone(), e))?;

		let index = match bitcode::deserialize(&serialized[..]) {
			Ok(i) => i,
			Err(_) => return Err(Error::IndexDeserializationError),
		};

		self.replace_index(index).await;

		Ok(true)
	}

	pub async fn browse(&self, virtual_path: PathBuf) -> Result<Vec<browser::File>, Error> {
		spawn_blocking({
			let index_manager = self.clone();
			move || {
				let index = index_manager.index.read().unwrap();
				index.browser.browse(&index.dictionary, virtual_path)
			}
		})
		.await
		.unwrap()
	}

	pub async fn flatten(&self, virtual_path: PathBuf) -> Result<Vec<PathBuf>, Error> {
		spawn_blocking({
			let index_manager = self.clone();
			move || {
				let index = index_manager.index.read().unwrap();
				index.browser.flatten(&index.dictionary, virtual_path)
			}
		})
		.await
		.unwrap()
	}

	pub async fn get_genres(&self) -> Vec<GenreHeader> {
		spawn_blocking({
			let index_manager = self.clone();
			move || {
				let index = index_manager.index.read().unwrap();
				index.collection.get_genres(&index.dictionary)
			}
		})
		.await
		.unwrap()
	}

	pub async fn get_genre(&self, name: String) -> Result<Genre, Error> {
		spawn_blocking({
			let index_manager = self.clone();
			move || {
				let index = index_manager.index.read().unwrap();
				let name = index
					.dictionary
					.get(&name)
					.ok_or_else(|| Error::GenreNotFound)?;
				let genre_key = GenreKey(name);
				index
					.collection
					.get_genre(&index.dictionary, genre_key)
					.ok_or_else(|| Error::GenreNotFound)
			}
		})
		.await
		.unwrap()
	}

	pub async fn get_albums(&self) -> Vec<AlbumHeader> {
		spawn_blocking({
			let index_manager = self.clone();
			move || {
				let index = index_manager.index.read().unwrap();
				index.collection.get_albums(&index.dictionary)
			}
		})
		.await
		.unwrap()
	}

	pub async fn get_artists(&self) -> Vec<ArtistHeader> {
		spawn_blocking({
			let index_manager = self.clone();
			move || {
				let index = index_manager.index.read().unwrap();
				index.collection.get_artists(&index.dictionary)
			}
		})
		.await
		.unwrap()
	}

	pub async fn get_artist(&self, name: String) -> Result<Artist, Error> {
		spawn_blocking({
			let index_manager = self.clone();
			move || {
				let index = index_manager.index.read().unwrap();
				let name = index
					.dictionary
					.get(name)
					.ok_or_else(|| Error::ArtistNotFound)?;
				let artist_key = ArtistKey(name);
				index
					.collection
					.get_artist(&index.dictionary, artist_key)
					.ok_or_else(|| Error::ArtistNotFound)
			}
		})
		.await
		.unwrap()
	}

	pub async fn get_album(&self, artists: Vec<String>, name: String) -> Result<Album, Error> {
		spawn_blocking({
			let index_manager = self.clone();
			move || {
				let index = index_manager.index.read().unwrap();
				let name = index
					.dictionary
					.get(&name)
					.ok_or_else(|| Error::AlbumNotFound)?;
				let album_key = AlbumKey {
					artists: artists
						.into_iter()
						.filter_map(|a| index.dictionary.get(a))
						.map(ArtistKey)
						.collect(),
					name,
				};
				index
					.collection
					.get_album(&index.dictionary, album_key)
					.ok_or_else(|| Error::AlbumNotFound)
			}
		})
		.await
		.unwrap()
	}

	pub async fn get_random_albums(
		&self,
		seed: Option<u64>,
		offset: usize,
		count: usize,
	) -> Result<Vec<Album>, Error> {
		spawn_blocking({
			let index_manager = self.clone();
			move || {
				let index = index_manager.index.read().unwrap();
				Ok(index
					.collection
					.get_random_albums(&index.dictionary, seed, offset, count))
			}
		})
		.await
		.unwrap()
	}

	pub async fn get_recent_albums(
		&self,
		offset: usize,
		count: usize,
	) -> Result<Vec<Album>, Error> {
		spawn_blocking({
			let index_manager = self.clone();
			move || {
				let index = index_manager.index.read().unwrap();
				Ok(index
					.collection
					.get_recent_albums(&index.dictionary, offset, count))
			}
		})
		.await
		.unwrap()
	}

	pub async fn get_songs(&self, virtual_paths: Vec<PathBuf>) -> Vec<Result<Song, Error>> {
		spawn_blocking({
			let index_manager = self.clone();
			move || {
				let index = index_manager.index.read().unwrap();
				virtual_paths
					.into_iter()
					.map(|p| {
						p.get(&index.dictionary)
							.and_then(|virtual_path| {
								let key = SongKey { virtual_path };
								index.collection.get_song(&index.dictionary, key)
							})
							.ok_or_else(|| Error::SongNotFound)
					})
					.collect()
			}
		})
		.await
		.unwrap()
	}

	pub async fn search(&self, query: String) -> Result<Vec<Song>, Error> {
		spawn_blocking({
			let index_manager = self.clone();
			move || {
				let index = index_manager.index.read().unwrap();
				index
					.search
					.find_songs(&index.collection, &index.dictionary, &query)
			}
		})
		.await
		.unwrap()
	}
}

#[derive(Serialize, Deserialize)]
pub struct Index {
	pub dictionary: dictionary::Dictionary,
	pub browser: browser::Browser,
	pub collection: collection::Collection,
	pub search: search::Search,
}

impl Default for Index {
	fn default() -> Self {
		Self {
			dictionary: Default::default(),
			browser: Default::default(),
			collection: Default::default(),
			search: Default::default(),
		}
	}
}

#[derive(Clone)]
pub struct Builder {
	dictionary_builder: dictionary::Builder,
	browser_builder: browser::Builder,
	collection_builder: collection::Builder,
	search_builder: search::Builder,
}

impl Builder {
	pub fn new() -> Self {
		Self {
			dictionary_builder: dictionary::Builder::default(),
			browser_builder: browser::Builder::default(),
			collection_builder: collection::Builder::default(),
			search_builder: search::Builder::default(),
		}
	}

	pub fn add_directory(&mut self, directory: scanner::Directory) {
		self.browser_builder
			.add_directory(&mut self.dictionary_builder, directory);
	}

	pub fn add_song(&mut self, scanner_song: scanner::Song) {
		if let Some(storage_song) = store_song(&mut self.dictionary_builder, &scanner_song) {
			self.browser_builder
				.add_song(&mut self.dictionary_builder, &scanner_song);
			self.collection_builder.add_song(&storage_song);
			self.search_builder.add_song(&scanner_song, &storage_song);
		}
	}

	pub fn build(self) -> Index {
		Index {
			dictionary: self.dictionary_builder.build(),
			browser: self.browser_builder.build(),
			collection: self.collection_builder.build(),
			search: self.search_builder.build(),
		}
	}
}

impl Default for Builder {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod test {
	use crate::{
		app::{index, test},
		test_name,
	};

	#[tokio::test]
	async fn can_persist_index() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;
		assert_eq!(ctx.index_manager.try_restore_index().await.unwrap(), false);
		let index = index::Builder::new().build();
		ctx.index_manager.persist_index(&index).await.unwrap();
		assert_eq!(ctx.index_manager.try_restore_index().await.unwrap(), true);
	}
}
