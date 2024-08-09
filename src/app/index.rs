use std::{
	path::{Path, PathBuf},
	sync::{Arc, RwLock},
};

use lasso2::ThreadedRodeo;
use log::{error, info};
use serde::{Deserialize, Serialize};
use tokio::task::spawn_blocking;

use crate::app::{scanner, Error};
use crate::db::DB;

mod browser;
mod collection;
mod search;

pub use browser::File;
pub use collection::{Album, AlbumKey, Artist, ArtistKey, Song, SongKey};

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

	pub async fn get_artist(&self, artist_key: &ArtistKey) -> Result<Artist, Error> {
		spawn_blocking({
			let index_manager = self.clone();
			let artist_id = artist_key.into();
			move || {
				let index = index_manager.index.read().unwrap();
				index
					.collection
					.get_artist(artist_id)
					.ok_or_else(|| Error::ArtistNotFound)
			}
		})
		.await
		.unwrap()
	}

	pub async fn get_album(&self, album_key: &AlbumKey) -> Result<Album, Error> {
		spawn_blocking({
			let index_manager = self.clone();
			let album_id = album_key.into();
			move || {
				let index = index_manager.index.read().unwrap();
				index
					.collection
					.get_album(album_id)
					.ok_or_else(|| Error::AlbumNotFound)
			}
		})
		.await
		.unwrap()
	}

	pub async fn get_random_albums(&self, count: usize) -> Result<Vec<Album>, Error> {
		spawn_blocking({
			let index_manager = self.clone();
			move || {
				let index = index_manager.index.read().unwrap();
				Ok(index.collection.get_random_albums(count))
			}
		})
		.await
		.unwrap()
	}

	pub async fn get_recent_albums(&self, count: usize) -> Result<Vec<Album>, Error> {
		spawn_blocking({
			let index_manager = self.clone();
			move || {
				let index = index_manager.index.read().unwrap();
				Ok(index.collection.get_recent_albums(count))
			}
		})
		.await
		.unwrap()
	}

	pub async fn get_song(&self, song_key: &SongKey) -> Result<Song, Error> {
		spawn_blocking({
			let index_manager = self.clone();
			let song_id = song_key.into();
			move || {
				let index = index_manager.index.read().unwrap();
				index
					.collection
					.get_song(song_id)
					.ok_or_else(|| Error::SongNotFound)
			}
		})
		.await
		.unwrap()
	}

	pub async fn search(&self, _query: &str) -> Result<Vec<PathBuf>, Error> {
		todo!();
	}
}

#[derive(Default, Serialize, Deserialize)]
pub struct Index {
	pub strings: ThreadedRodeo,
	pub browser: browser::Browser,
	pub collection: collection::Collection,
}

pub struct Builder {
	strings: ThreadedRodeo,
	browser_builder: browser::Builder,
	collection_builder: collection::Builder,
}

impl Builder {
	pub fn new() -> Self {
		Self {
			strings: ThreadedRodeo::new(),
			browser_builder: browser::Builder::default(),
			collection_builder: collection::Builder::default(),
		}
	}

	pub fn add_directory(&mut self, directory: scanner::Directory) {
		self.browser_builder
			.add_directory(&mut self.strings, directory);
	}

	pub fn add_song(&mut self, song: scanner::Song) {
		self.browser_builder.add_song(&mut self.strings, &song);
		self.collection_builder.add_song(song);
	}

	pub fn build(mut self) -> Index {
		Index {
			browser: self.browser_builder.build(&mut self.strings),
			collection: self.collection_builder.build(),
			strings: self.strings,
		}
	}
}

impl Default for Builder {
	fn default() -> Self {
		Self::new()
	}
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub(self) struct PathID(lasso2::Spur);

pub(self) trait InternPath {
	fn get_or_intern(self, strings: &mut ThreadedRodeo) -> Option<PathID>;
	fn get(self, strings: &ThreadedRodeo) -> Option<PathID>;
}

impl<P: AsRef<Path>> InternPath for P {
	fn get_or_intern(self, strings: &mut ThreadedRodeo) -> Option<PathID> {
		let id = self
			.as_ref()
			.as_os_str()
			.to_str()
			.map(|s| strings.get_or_intern(s))
			.map(PathID);
		if id.is_none() {
			error!("Unsupported path: `{}`", self.as_ref().to_string_lossy());
		}
		id
	}

	fn get(self, strings: &ThreadedRodeo) -> Option<PathID> {
		let id = self
			.as_ref()
			.as_os_str()
			.to_str()
			.and_then(|s| strings.get(s))
			.map(PathID);
		if id.is_none() {
			error!("Unsupported path: `{}`", self.as_ref().to_string_lossy());
		}
		id
	}
}
