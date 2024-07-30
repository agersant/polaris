use std::{
	collections::{HashMap, HashSet},
	hash::{DefaultHasher, Hash, Hasher},
	sync::Arc,
};

use log::{error, info};
use rand::{rngs::ThreadRng, seq::IteratorRandom};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{app::collection, db::DB};

#[derive(Clone)]
pub struct IndexManager {
	db: DB,
	index: Arc<RwLock<Index>>,
}

impl IndexManager {
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

	pub(super) async fn replace_index(&mut self, new_index: Index) {
		let mut lock = self.index.write().await;
		*lock = new_index;
	}

	pub(super) async fn persist_index(&mut self, index: &Index) -> Result<(), collection::Error> {
		let serialized = match bitcode::serialize(index) {
			Ok(s) => s,
			Err(_) => return Err(collection::Error::IndexSerializationError),
		};
		sqlx::query!("UPDATE collection_index SET content = $1", serialized)
			.execute(self.db.connect().await?.as_mut())
			.await?;
		Ok(())
	}

	async fn try_restore_index(&mut self) -> Result<bool, collection::Error> {
		let serialized = sqlx::query_scalar!("SELECT content FROM collection_index")
			.fetch_one(self.db.connect().await?.as_mut())
			.await?;

		let Some(serialized) = serialized else {
			info!("Database did not contain a collection to restore");
			return Ok(false);
		};

		let index = match bitcode::deserialize(&serialized[..]) {
			Ok(i) => i,
			Err(_) => return Err(collection::Error::IndexDeserializationError),
		};

		self.replace_index(index).await;

		Ok(true)
	}

	pub async fn get_random_albums(
		&self,
		count: usize,
	) -> Result<Vec<collection::Album>, collection::Error> {
		let index = self.index.read().await;
		Ok(index
			.albums
			.keys()
			.choose_multiple(&mut ThreadRng::default(), count)
			.into_iter()
			.filter_map(|k| index.get_album(*k))
			.collect())
	}

	pub async fn get_recent_albums(
		&self,
		count: usize,
	) -> Result<Vec<collection::Album>, collection::Error> {
		let index = self.index.read().await;
		Ok(index
			.recent_albums
			.iter()
			.take(count)
			.filter_map(|k| index.get_album(*k))
			.collect())
	}
}

#[derive(Default)]
pub(super) struct IndexBuilder {
	songs: HashMap<SongID, collection::Song>,
	albums: HashMap<AlbumID, Album>,
}

impl IndexBuilder {
	pub fn add_song(&mut self, song: collection::Song) {
		let song_id: SongID = song.song_id();
		self.add_song_to_album(&song);
		self.songs.insert(song_id, song);
	}

	fn add_song_to_album(&mut self, song: &collection::Song) {
		let song_id: SongID = song.song_id();
		let album_id: AlbumID = song.album_id();

		let album = match self.albums.get_mut(&album_id) {
			Some(l) => l,
			None => {
				self.albums.insert(album_id, Album::default());
				self.albums.get_mut(&album_id).unwrap()
			}
		};

		if album.name.is_none() {
			album.name = song.album.clone();
		}

		if album.artwork.is_none() {
			album.artwork = song.artwork.clone();
		}

		if album.year.is_none() {
			album.year = song.year.clone();
		}

		album.date_added = album.date_added.min(song.date_added);

		if !song.album_artists.0.is_empty() {
			album.artists = song.album_artists.0.clone();
		} else if !song.album_artists.0.is_empty() {
			album.artists = song.artists.0.clone();
		}

		album.songs.insert(song_id);
	}

	pub fn build(self) -> Index {
		let mut recent_albums = self.albums.keys().cloned().collect::<Vec<_>>();
		recent_albums.sort_by_key(|a| {
			self.albums
				.get(a)
				.map(|a| -a.date_added)
				.unwrap_or_default()
		});

		Index {
			songs: self.songs,
			albums: self.albums,
			recent_albums,
		}
	}
}

#[derive(Default, Serialize, Deserialize)]
pub(super) struct Index {
	songs: HashMap<SongID, collection::Song>,
	albums: HashMap<AlbumID, Album>,
	recent_albums: Vec<AlbumID>,
}

impl Index {
	pub fn get_album(&self, album_id: AlbumID) -> Option<collection::Album> {
		self.albums.get(&album_id).map(|a| {
			let songs = a
				.songs
				.iter()
				.filter_map(|s| self.songs.get(s))
				.cloned()
				.collect::<Vec<_>>();

			collection::Album {
				name: a.name.clone(),
				artwork: a.artwork.clone(),
				artists: a.artists.clone(),
				year: a.year,
				date_added: a.date_added,
				songs,
			}
		})
	}
}

#[derive(Clone, Copy, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct SongID(u64);

#[derive(Clone, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct SongKey {
	pub virtual_path: String,
}

impl From<&collection::Song> for SongKey {
	fn from(song: &collection::Song) -> Self {
		SongKey {
			virtual_path: song.virtual_path.clone(),
		}
	}
}

impl From<&SongKey> for SongID {
	fn from(key: &SongKey) -> Self {
		SongID(key.id())
	}
}

impl collection::Song {
	pub(self) fn song_id(&self) -> SongID {
		let key: SongKey = self.into();
		(&key).into()
	}
}

#[derive(Default, Serialize, Deserialize)]
struct Album {
	pub name: Option<String>,
	pub artwork: Option<String>,
	pub artists: Vec<String>,
	pub year: Option<i64>,
	pub date_added: i64,
	pub songs: HashSet<SongID>,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct AlbumID(u64);

#[derive(Clone, Eq, Hash, PartialEq)]
struct AlbumKey {
	pub artists: Vec<String>,
	pub name: Option<String>,
}

impl From<&collection::Song> for AlbumKey {
	fn from(song: &collection::Song) -> Self {
		let album_artists = match song.album_artists.0.is_empty() {
			true => &song.artists.0,
			false => &song.album_artists.0,
		};

		AlbumKey {
			artists: album_artists.iter().cloned().collect(),
			name: song.album.clone(),
		}
	}
}

impl From<&AlbumKey> for AlbumID {
	fn from(key: &AlbumKey) -> Self {
		AlbumID(key.id())
	}
}

impl collection::Song {
	pub(self) fn album_id(&self) -> AlbumID {
		let key: AlbumKey = self.into();
		(&key).into()
	}
}

trait ID {
	fn id(&self) -> u64;
}

impl<T: Hash> ID for T {
	fn id(&self) -> u64 {
		let mut hasher = DefaultHasher::default();
		self.hash(&mut hasher);
		hasher.finish()
	}
}

#[cfg(test)]
mod test {

	use crate::app::test;
	use crate::test_name;

	const TEST_MOUNT_NAME: &str = "root";

	#[tokio::test]
	async fn can_get_random_albums() {
		let mut ctx = test::ContextBuilder::new(test_name!())
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build()
			.await;
		ctx.updater.update().await.unwrap();
		let albums = ctx.index_manager.get_random_albums(1).await.unwrap();
		assert_eq!(albums.len(), 1);
	}

	#[tokio::test]
	async fn can_get_recent_albums() {
		let mut ctx = test::ContextBuilder::new(test_name!())
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build()
			.await;
		ctx.updater.update().await.unwrap();
		let albums = ctx.index_manager.get_recent_albums(2).await.unwrap();
		assert_eq!(albums.len(), 2);
	}
}
