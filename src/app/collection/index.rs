use std::{
	borrow::BorrowMut,
	collections::{HashMap, HashSet},
	hash::{DefaultHasher, Hash, Hasher},
	path::{Path, PathBuf},
	sync::{Arc, RwLock},
};

use log::{error, info};
use rand::{rngs::ThreadRng, seq::IteratorRandom};
use serde::{Deserialize, Serialize};
use tokio::task::spawn_blocking;

use crate::{app::collection, db::DB};

#[derive(Clone)]
pub struct IndexManager {
	db: DB,
	index: Arc<RwLock<Index>>, // Not a tokio RwLock as we want to do CPU-bound work with Index
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

	pub async fn browse(
		&self,
		virtual_path: PathBuf,
	) -> Result<Vec<collection::File>, collection::Error> {
		spawn_blocking({
			let index_manager = self.clone();
			move || {
				let index = index_manager.index.read().unwrap();
				index.browse(virtual_path)
			}
		})
		.await
		.unwrap()
	}
	pub async fn get_artist(
		&self,
		artist_key: &ArtistKey,
	) -> Result<collection::Artist, collection::Error> {
		spawn_blocking({
			let index_manager = self.clone();
			let artist_id = artist_key.into();
			move || {
				let index = index_manager.index.read().unwrap();
				index
					.get_artist(artist_id)
					.ok_or_else(|| collection::Error::ArtistNotFound)
			}
		})
		.await
		.unwrap()
	}

	pub async fn get_album(
		&self,
		album_key: &AlbumKey,
	) -> Result<collection::Album, collection::Error> {
		spawn_blocking({
			let index_manager = self.clone();
			let album_id = album_key.into();
			move || {
				let index = index_manager.index.read().unwrap();
				index
					.get_album(album_id)
					.ok_or_else(|| collection::Error::AlbumNotFound)
			}
		})
		.await
		.unwrap()
	}

	pub async fn get_random_albums(
		&self,
		count: usize,
	) -> Result<Vec<collection::Album>, collection::Error> {
		spawn_blocking({
			let index_manager = self.clone();
			move || {
				let index = index_manager.index.read().unwrap();
				Ok(index
					.albums
					.keys()
					.choose_multiple(&mut ThreadRng::default(), count)
					.into_iter()
					.filter_map(|k| index.get_album(*k))
					.collect())
			}
		})
		.await
		.unwrap()
	}

	pub async fn get_recent_albums(
		&self,
		count: usize,
	) -> Result<Vec<collection::Album>, collection::Error> {
		spawn_blocking({
			let index_manager = self.clone();
			move || {
				let index = index_manager.index.read().unwrap();
				Ok(index
					.recent_albums
					.iter()
					.take(count)
					.filter_map(|k| index.get_album(*k))
					.collect())
			}
		})
		.await
		.unwrap()
	}
}

#[derive(Default)]
pub(super) struct IndexBuilder {
	directories: HashMap<PathBuf, HashSet<collection::File>>,
	// filesystem: Trie<>,
	songs: HashMap<SongID, collection::Song>,
	artists: HashMap<ArtistID, Artist>,
	albums: HashMap<AlbumID, Album>,
}

impl IndexBuilder {
	pub fn add_directory(&mut self, directory: collection::Directory) {
		self.directories
			.entry(directory.virtual_path.clone())
			.or_default();
		if let Some(parent) = directory.virtual_parent {
			self.directories
				.entry(parent.clone())
				.or_default()
				.insert(collection::File::Directory(directory.virtual_path));
		}
	}

	pub fn add_song(&mut self, song: collection::Song) {
		let song_id: SongID = song.song_id();
		self.directories
			.entry(song.virtual_parent.clone())
			.or_default()
			.insert(collection::File::Song(song.virtual_path.clone()));
		self.add_song_to_album(&song);
		self.add_album_to_artists(&song);
		self.songs.insert(song_id, song);
	}

	fn add_album_to_artists(&mut self, song: &collection::Song) {
		let album_id: AlbumID = song.album_id();

		for artist_name in &song.album_artists {
			let artist = self.get_or_create_artist(artist_name);
			artist.albums.insert(album_id);
		}

		for artist_name in &song.artists {
			let artist = self.get_or_create_artist(artist_name);
			if song.album_artists.is_empty() {
				artist.albums.insert(album_id);
			} else if !song.album_artists.contains(artist_name) {
				artist.album_appearances.insert(album_id);
			}
		}
	}

	fn get_or_create_artist(&mut self, name: &String) -> &mut Artist {
		let artist_key = ArtistKey {
			name: Some(name.clone()),
		};
		let artist_id: ArtistID = (&artist_key).into();
		self.artists
			.entry(artist_id)
			.or_insert_with(|| Artist {
				name: Some(name.clone()),
				albums: HashSet::new(),
				album_appearances: HashSet::new(),
			})
			.borrow_mut()
	}

	fn add_song_to_album(&mut self, song: &collection::Song) {
		let song_id: SongID = song.song_id();
		let album_id: AlbumID = song.album_id();

		let album = self.albums.entry(album_id).or_default().borrow_mut();

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

		if !song.album_artists.is_empty() {
			album.artists = song.album_artists.clone();
		} else if !song.artists.is_empty() {
			album.artists = song.artists.clone();
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
			directories: self.directories,
			songs: self.songs,
			artists: self.artists,
			albums: self.albums,
			recent_albums,
		}
	}
}

#[derive(Default, Serialize, Deserialize)]
pub(super) struct Index {
	directories: HashMap<PathBuf, HashSet<collection::File>>,
	songs: HashMap<SongID, collection::Song>,
	artists: HashMap<ArtistID, Artist>,
	albums: HashMap<AlbumID, Album>,
	recent_albums: Vec<AlbumID>,
}

impl Index {
	pub(self) fn browse<P: AsRef<Path>>(
		&self,
		virtual_path: P,
	) -> Result<Vec<collection::File>, collection::Error> {
		let Some(files) = self.directories.get(virtual_path.as_ref()) else {
			return Err(collection::Error::DirectoryNotFound(
				virtual_path.as_ref().to_owned(),
			));
		};
		Ok(files.iter().cloned().collect())
	}

	pub(self) fn get_artist(&self, artist_id: ArtistID) -> Option<collection::Artist> {
		self.artists.get(&artist_id).map(|a| {
			let albums = {
				let mut albums = a
					.albums
					.iter()
					.filter_map(|album_id| self.get_album(*album_id))
					.collect::<Vec<_>>();
				albums.sort_by(|a, b| (a.year, &a.name).partial_cmp(&(b.year, &b.name)).unwrap());
				albums
			};

			let album_appearances = {
				let mut album_appearances = a
					.album_appearances
					.iter()
					.filter_map(|album_id| self.get_album(*album_id))
					.collect::<Vec<_>>();
				album_appearances.sort_by(|a, b| {
					(&a.artists, a.year, &a.name)
						.partial_cmp(&(&b.artists, b.year, &b.name))
						.unwrap()
				});
				album_appearances
			};

			collection::Artist {
				name: a.name.clone(),
				albums,
				album_appearances,
			}
		})
	}

	pub(self) fn get_album(&self, album_id: AlbumID) -> Option<collection::Album> {
		self.albums.get(&album_id).map(|a| {
			let mut songs = a
				.songs
				.iter()
				.filter_map(|s| self.songs.get(s))
				.cloned()
				.collect::<Vec<_>>();

			songs.sort_by_key(|s| (s.disc_number.unwrap_or(-1), s.track_number.unwrap_or(-1)));

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
struct SongID(u64);

#[derive(Clone, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct SongKey {
	pub virtual_path: PathBuf,
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

#[derive(Serialize, Deserialize)]
struct Artist {
	pub name: Option<String>,
	pub albums: HashSet<AlbumID>,
	pub album_appearances: HashSet<AlbumID>,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq, Serialize, Deserialize)]
struct ArtistID(u64);

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct ArtistKey {
	pub name: Option<String>,
}

impl From<&ArtistKey> for ArtistID {
	fn from(key: &ArtistKey) -> Self {
		ArtistID(key.id())
	}
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct Album {
	pub name: Option<String>,
	pub artwork: Option<PathBuf>,
	pub artists: Vec<String>,
	pub year: Option<i64>,
	pub date_added: i64,
	pub songs: HashSet<SongID>,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq, Serialize, Deserialize)]
struct AlbumID(u64);

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct AlbumKey {
	pub artists: Vec<String>,
	pub name: Option<String>,
}

impl From<&collection::Song> for AlbumKey {
	fn from(song: &collection::Song) -> Self {
		let album_artists = match song.album_artists.is_empty() {
			true => &song.artists,
			false => &song.album_artists,
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
