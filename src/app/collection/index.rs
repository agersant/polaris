use std::{
	collections::{HashMap, HashSet},
	hash::{DefaultHasher, Hash, Hasher},
	sync::Arc,
};

use rand::{rngs::ThreadRng, seq::IteratorRandom};
use tokio::sync::RwLock;

use crate::app::collection;

#[derive(Clone)]
pub struct IndexManager {
	index: Arc<RwLock<Index>>,
}

impl IndexManager {
	pub fn new() -> Self {
		Self {
			index: Arc::default(),
		}
	}

	pub(super) async fn replace_index(&mut self, new_index: Index) {
		let mut lock = self.index.write().await;
		*lock = new_index;
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
	pub fn add_song(&mut self, song: &collection::Song) {
		self.songs.insert(song.into(), song.clone());
		self.add_song_to_album(song);
	}

	fn add_song_to_album(&mut self, song: &collection::Song) {
		let album_id: AlbumID = song.into();

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

		album.songs.insert(song.into());
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

#[derive(Default)]
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

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct SongID(u64);

#[derive(Clone, Eq, Hash, PartialEq)]
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
		let mut hasher = DefaultHasher::default();
		key.hash(&mut hasher);
		SongID(hasher.finish())
	}
}

impl From<&collection::Song> for SongID {
	fn from(song: &collection::Song) -> Self {
		let key: SongKey = song.into();
		(&key).into()
	}
}

#[derive(Default)]
struct Album {
	pub name: Option<String>,
	pub artwork: Option<String>,
	pub artists: Vec<String>,
	pub year: Option<i64>,
	pub date_added: i64,
	pub songs: HashSet<SongID>,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
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
		let mut hasher = DefaultHasher::default();
		key.hash(&mut hasher);
		AlbumID(hasher.finish())
	}
}

impl From<&collection::Song> for AlbumID {
	fn from(song: &collection::Song) -> Self {
		let key: AlbumKey = song.into();
		(&key).into()
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
