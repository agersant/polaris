use std::{
	collections::{HashMap, HashSet},
	sync::Arc,
};

use rand::{rngs::ThreadRng, seq::IteratorRandom};
use tokio::sync::RwLock;

use crate::app::collection;

#[derive(Clone)]
pub struct Index {
	lookups: Arc<RwLock<Lookups>>,
}

impl Index {
	pub fn new() -> Self {
		Self {
			lookups: Arc::default(),
		}
	}

	pub(super) async fn replace_lookup_tables(&mut self, new_lookups: Lookups) {
		let mut lock = self.lookups.write().await;
		*lock = new_lookups;
	}

	pub async fn get_random_albums(
		&self,
		count: usize,
	) -> Result<Vec<collection::Album>, collection::Error> {
		let lookups = self.lookups.read().await;
		Ok(lookups
			.songs_by_albums
			.keys()
			.choose_multiple(&mut ThreadRng::default(), count)
			.iter()
			.filter_map(|k| lookups.get_album(k))
			.collect())
	}

	pub async fn get_recent_albums(
		&self,
		count: i64,
	) -> Result<Vec<collection::Album>, collection::Error> {
		// TODO implement
		Ok(vec![])
	}
}

// TODO how can clients refer to an album?
#[derive(Clone, PartialEq, Eq, Hash)]
struct AlbumKey {
	pub artists: Vec<String>,
	pub name: Option<String>,
}

#[derive(Default)]
pub(super) struct Lookups {
	all_songs: HashMap<String, collection::Song>,
	songs_by_albums: HashMap<AlbumKey, HashSet<String>>, // TODO should this store collection::Album structs instead?
}

impl Lookups {
	pub fn add_song(&mut self, song: &collection::Song) {
		self.all_songs
			.insert(song.virtual_path.clone(), song.clone());

		let album_artists = match song.album_artists.0.is_empty() {
			true => &song.artists.0,
			false => &song.album_artists.0,
		};

		let album_key = AlbumKey {
			artists: album_artists.iter().cloned().collect(),
			name: song.album.clone(),
		};

		let song_list = match self.songs_by_albums.get_mut(&album_key) {
			Some(l) => l,
			None => {
				self.songs_by_albums
					.insert(album_key.clone(), HashSet::new());
				self.songs_by_albums.get_mut(&album_key).unwrap()
			}
		};

		song_list.insert(song.virtual_path.clone());
	}

	pub fn get_album(&self, key: &AlbumKey) -> Option<collection::Album> {
		let Some(songs) = self.songs_by_albums.get(key) else {
			return None;
		};

		let songs: Vec<&collection::Song> =
			songs.iter().filter_map(|s| self.all_songs.get(s)).collect();

		Some(collection::Album {
			name: key.name.clone(),
			artwork: songs.iter().find_map(|s| s.artwork.clone()),
			artists: key.artists.iter().cloned().collect(),
			year: songs.iter().find_map(|s| s.year),
			date_added: songs
				.iter()
				.min_by_key(|s| s.date_added)
				.map(|s| s.date_added)
				.unwrap_or_default(),
		})
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
		let albums = ctx.index.get_random_albums(1).await.unwrap();
		assert_eq!(albums.len(), 1);
	}

	#[tokio::test]
	async fn can_get_recent_albums() {
		let mut ctx = test::ContextBuilder::new(test_name!())
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build()
			.await;
		ctx.updater.update().await.unwrap();
		let albums = ctx.index.get_recent_albums(2).await.unwrap();
		assert_eq!(albums.len(), 2);
	}
}
