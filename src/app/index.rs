use std::{
	borrow::Borrow,
	collections::HashMap,
	path::PathBuf,
	sync::{Arc, RwLock},
};

use lasso2::{Rodeo, RodeoReader, Spur};
use log::{error, info};
use serde::{Deserialize, Serialize};
use tokio::task::spawn_blocking;

use crate::app::{scanner, Error};
use crate::db::DB;

mod browser;
mod collection;
mod query;
mod search;
mod storage;

pub use browser::File;
pub use collection::{Album, AlbumHeader, Artist, ArtistHeader, Song};
use storage::{store_song, AlbumKey, ArtistKey, InternPath, SongKey};

#[derive(Clone)]
pub struct Manager {
	db: DB,
	index: Arc<RwLock<Index>>, // Not a tokio RwLock as we want to do CPU-bound work with Index
}

impl Manager {
	pub async fn new(db: DB) -> Self {
		let mut index_manager = Self {
			db,
			index: Arc::default(),
		};
		if let Err(e) = index_manager.try_restore_index().await {
			error!("Failed to restore index: {}", e);
		}
		index_manager
	}

	pub async fn replace_index(&mut self, new_index: Index) {
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

	pub async fn persist_index(&mut self, index: &Index) -> Result<(), Error> {
		let serialized = match bitcode::serialize(index) {
			Ok(s) => s,
			Err(_) => return Err(Error::IndexSerializationError),
		};
		sqlx::query!("UPDATE collection_index SET content = $1", serialized)
			.execute(self.db.connect().await?.as_mut())
			.await?;
		Ok(())
	}

	async fn try_restore_index(&mut self) -> Result<bool, Error> {
		let serialized = sqlx::query_scalar!("SELECT content FROM collection_index")
			.fetch_one(self.db.connect().await?.as_mut())
			.await?;

		let Some(serialized) = serialized else {
			info!("Database did not contain a collection to restore");
			return Ok(false);
		};

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
				index.browser.browse(&index.strings, virtual_path)
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
				index.browser.flatten(&index.strings, virtual_path)
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
				index.collection.get_albums(&index.strings)
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
				index.collection.get_artists(&index.strings)
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
				let artist_key = ArtistKey {
					name: match name.as_str() {
						"" => None,
						s => index.strings.get(s),
					},
				};
				index
					.collection
					.get_artist(&index.strings, artist_key)
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
					.strings
					.get(&name)
					.ok_or_else(|| Error::AlbumNotFound)?;
				let album_key = AlbumKey {
					artists: artists
						.into_iter()
						.filter_map(|a| index.strings.get(a))
						.collect(),
					name,
				};
				index
					.collection
					.get_album(&index.strings, album_key)
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
					.get_random_albums(&index.strings, seed, offset, count))
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
					.get_recent_albums(&index.strings, offset, count))
			}
		})
		.await
		.unwrap()
	}

	fn get_song_internal(virtual_path: &PathBuf, index: &Index) -> Result<Song, Error> {
		let Some(virtual_path) = virtual_path.get(&index.strings) else {
			return Err(Error::SongNotFound);
		};
		let song_key = SongKey { virtual_path };
		index
			.collection
			.get_song(&index.strings, song_key)
			.ok_or_else(|| Error::SongNotFound)
	}

	pub async fn get_song(&self, virtual_path: PathBuf) -> Result<Song, Error> {
		spawn_blocking({
			let index_manager = self.clone();
			move || {
				let index = index_manager.index.read().unwrap();
				Self::get_song_internal(&virtual_path, index.borrow())
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
					.map(|path| Self::get_song_internal(&path, index.borrow()))
					.collect()
			}
		})
		.await
		.unwrap()
	}

	pub async fn search(&self, query: String) -> Result<Vec<PathBuf>, Error> {
		spawn_blocking({
			let index_manager = self.clone();
			move || {
				let index = index_manager.index.read().unwrap();
				index
					.search
					.find_songs(&index.strings, &index.canon, &query)
			}
		})
		.await
		.unwrap()
	}
}

#[derive(Serialize, Deserialize)]
pub struct Index {
	pub strings: RodeoReader,
	pub canon: HashMap<String, Spur>,
	pub browser: browser::Browser,
	pub collection: collection::Collection,
	pub search: search::Search,
}

impl Default for Index {
	fn default() -> Self {
		Self {
			strings: Rodeo::new().into_reader(),
			canon: Default::default(),
			browser: Default::default(),
			collection: Default::default(),
			search: Default::default(),
		}
	}
}

pub struct Builder {
	strings: Rodeo,
	canon: HashMap<String, Spur>,
	browser_builder: browser::Builder,
	collection_builder: collection::Builder,
	search_builder: search::Builder,
}

impl Builder {
	pub fn new() -> Self {
		Self {
			strings: Rodeo::new(),
			canon: HashMap::default(),
			browser_builder: browser::Builder::default(),
			collection_builder: collection::Builder::default(),
			search_builder: search::Builder::default(),
		}
	}

	pub fn add_directory(&mut self, directory: scanner::Directory) {
		self.browser_builder
			.add_directory(&mut self.strings, directory);
	}

	pub fn add_song(&mut self, scanner_song: scanner::Song) {
		if let Some(storage_song) = store_song(&mut self.strings, &mut self.canon, &scanner_song) {
			self.browser_builder
				.add_song(&mut self.strings, &scanner_song);
			self.collection_builder.add_song(&storage_song);
			self.search_builder.add_song(&scanner_song, &storage_song);
		}
	}

	pub fn build(self) -> Index {
		Index {
			browser: self.browser_builder.build(),
			collection: self.collection_builder.build(),
			search: self.search_builder.build(),
			strings: self.strings.into_reader(),
			canon: self.canon,
		}
	}
}

impl Default for Builder {
	fn default() -> Self {
		Self::new()
	}
}
