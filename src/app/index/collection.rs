use std::{
	borrow::BorrowMut,
	collections::{HashMap, HashSet},
	hash::{DefaultHasher, Hash, Hasher},
	path::PathBuf,
};

use rand::{rngs::ThreadRng, seq::IteratorRandom};
use serde::{Deserialize, Serialize};

use crate::app::scanner;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Artist {
	pub name: Option<String>,
	pub albums: Vec<Album>,
	pub album_appearances: Vec<Album>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Album {
	pub name: Option<String>,
	pub artwork: Option<PathBuf>,
	pub artists: Vec<String>,
	pub year: Option<i64>,
	pub date_added: i64,
	pub songs: Vec<Song>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Song {
	pub path: PathBuf,
	pub virtual_path: PathBuf,
	pub virtual_parent: PathBuf,
	pub track_number: Option<i64>,
	pub disc_number: Option<i64>,
	pub title: Option<String>,
	pub artists: Vec<String>,
	pub album_artists: Vec<String>,
	pub year: Option<i64>,
	pub album: Option<String>,
	pub artwork: Option<PathBuf>,
	pub duration: Option<i64>,
	pub lyricists: Vec<String>,
	pub composers: Vec<String>,
	pub genres: Vec<String>,
	pub labels: Vec<String>,
	pub date_added: i64,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Collection {
	artists: HashMap<ArtistID, storage::Artist>,
	albums: HashMap<AlbumID, storage::Album>,
	songs: HashMap<SongID, Song>,
	recent_albums: Vec<AlbumID>,
}

impl Collection {
	pub fn get_artist(&self, artist_id: ArtistID) -> Option<Artist> {
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

			Artist {
				name: a.name.clone(),
				albums,
				album_appearances,
			}
		})
	}

	pub fn get_album(&self, album_id: AlbumID) -> Option<Album> {
		self.albums.get(&album_id).map(|a| {
			let mut songs = a
				.songs
				.iter()
				.filter_map(|s| self.songs.get(s))
				.cloned()
				.collect::<Vec<_>>();

			songs.sort_by_key(|s| (s.disc_number.unwrap_or(-1), s.track_number.unwrap_or(-1)));

			Album {
				name: a.name.clone(),
				artwork: a.artwork.clone(),
				artists: a.artists.clone(),
				year: a.year,
				date_added: a.date_added,
				songs,
			}
		})
	}

	pub fn get_random_albums(&self, count: usize) -> Vec<Album> {
		self.albums
			.keys()
			.choose_multiple(&mut ThreadRng::default(), count)
			.into_iter()
			.filter_map(|k| self.get_album(*k))
			.collect()
	}

	pub fn get_recent_albums(&self, count: usize) -> Vec<Album> {
		self.recent_albums
			.iter()
			.take(count)
			.filter_map(|k| self.get_album(*k))
			.collect()
	}

	pub fn get_song(&self, song_id: SongID) -> Option<Song> {
		self.songs.get(&song_id).cloned()
	}
}

#[derive(Default)]
pub struct Builder {
	artists: HashMap<ArtistID, storage::Artist>,
	albums: HashMap<AlbumID, storage::Album>,
	songs: HashMap<SongID, Song>,
}

impl Builder {
	pub fn add_song(&mut self, song: scanner::Song) {
		let song = Song {
			path: song.path,
			virtual_path: song.virtual_path,
			virtual_parent: song.virtual_parent,
			track_number: song.track_number,
			disc_number: song.disc_number,
			title: song.title,
			artists: song.artists,
			album_artists: song.album_artists,
			year: song.year,
			album: song.album,
			artwork: song.artwork,
			duration: song.duration,
			lyricists: song.lyricists,
			composers: song.composers,
			genres: song.genres,
			labels: song.labels,
			date_added: song.date_added,
		};

		let song_id: SongID = song.song_id();
		self.add_song_to_album(&song);
		self.add_album_to_artists(&song);
		self.songs.insert(song_id, song);
	}

	pub fn build(self) -> Collection {
		let mut recent_albums = self.albums.keys().cloned().collect::<Vec<_>>();
		recent_albums.sort_by_key(|a| {
			self.albums
				.get(a)
				.map(|a| -a.date_added)
				.unwrap_or_default()
		});

		Collection {
			artists: self.artists,
			albums: self.albums,
			songs: self.songs,
			recent_albums,
		}
	}

	fn add_album_to_artists(&mut self, song: &Song) {
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

	fn get_or_create_artist(&mut self, name: &String) -> &mut storage::Artist {
		let artist_key = ArtistKey {
			name: Some(name.clone()),
		};
		let artist_id: ArtistID = (&artist_key).into();
		self.artists
			.entry(artist_id)
			.or_insert_with(|| storage::Artist {
				name: Some(name.clone()),
				albums: HashSet::new(),
				album_appearances: HashSet::new(),
			})
			.borrow_mut()
	}

	fn add_song_to_album(&mut self, song: &Song) {
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
}

mod storage {
	use super::*;

	#[derive(Serialize, Deserialize)]
	pub struct Artist {
		pub name: Option<String>,
		pub albums: HashSet<AlbumID>,
		pub album_appearances: HashSet<AlbumID>,
	}

	#[derive(Clone, Default, Serialize, Deserialize)]
	pub struct Album {
		pub name: Option<String>,
		pub artwork: Option<PathBuf>,
		pub artists: Vec<String>,
		pub year: Option<i64>,
		pub date_added: i64,
		pub songs: HashSet<SongID>,
	}
}

#[derive(Clone, Copy, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct ArtistID(u64);

#[derive(Clone, Copy, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct AlbumID(u64);

#[derive(Clone, Copy, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct SongID(u64);

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct ArtistKey {
	pub name: Option<String>,
}

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct AlbumKey {
	pub artists: Vec<String>,
	pub name: Option<String>,
}

#[derive(Clone, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct SongKey {
	pub virtual_path: PathBuf,
}

impl Song {
	pub fn album_key(&self) -> AlbumKey {
		let album_artists = match self.album_artists.is_empty() {
			true => &self.artists,
			false => &self.album_artists,
		};

		AlbumKey {
			artists: album_artists.iter().cloned().collect(),
			name: self.album.clone(),
		}
	}
	pub fn album_id(&self) -> AlbumID {
		// TODO we .song_key is cloning names just so we can hash them! Slow!
		let key: AlbumKey = self.album_key();
		(&key).into()
	}

	pub fn song_key(&self) -> SongKey {
		SongKey {
			virtual_path: self.virtual_path.clone(),
		}
	}

	pub fn song_id(&self) -> SongID {
		// TODO we .song_key is cloning path just so we can hash it! Slow!
		let key: SongKey = self.song_key();
		(&key).into()
	}
}

impl From<&ArtistKey> for ArtistID {
	fn from(key: &ArtistKey) -> Self {
		ArtistID(key.id())
	}
}

impl From<&AlbumKey> for AlbumID {
	fn from(key: &AlbumKey) -> Self {
		AlbumID(key.id())
	}
}

impl From<&SongKey> for SongID {
	fn from(key: &SongKey) -> Self {
		SongID(key.id())
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

	use super::*;

	use crate::app::test;
	use crate::test_name;

	const TEST_MOUNT_NAME: &str = "root";

	#[tokio::test]
	async fn can_get_random_albums() {
		let mut ctx = test::ContextBuilder::new(test_name!())
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build()
			.await;
		ctx.scanner.update().await.unwrap();
		let albums = ctx.index_manager.get_random_albums(1).await.unwrap();
		assert_eq!(albums.len(), 1);
	}

	#[tokio::test]
	async fn can_get_recent_albums() {
		let mut ctx = test::ContextBuilder::new(test_name!())
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build()
			.await;
		ctx.scanner.update().await.unwrap();
		let albums = ctx.index_manager.get_recent_albums(2).await.unwrap();
		assert_eq!(albums.len(), 2);
	}

	#[tokio::test]
	async fn can_get_a_song() {
		let mut ctx = test::ContextBuilder::new(test_name!())
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build()
			.await;

		ctx.scanner.update().await.unwrap();

		let picnic_virtual_dir: PathBuf = [TEST_MOUNT_NAME, "Tobokegao", "Picnic"].iter().collect();
		let song_virtual_path = picnic_virtual_dir.join("05 - シャーベット (Sherbet).mp3");
		let artwork_virtual_path = picnic_virtual_dir.join("Folder.png");

		let song = ctx
			.index_manager
			.get_song(&SongKey {
				virtual_path: song_virtual_path.clone(),
			})
			.await
			.unwrap();
		assert_eq!(song.virtual_path, song_virtual_path);
		assert_eq!(song.track_number, Some(5));
		assert_eq!(song.disc_number, None);
		assert_eq!(song.title, Some("シャーベット (Sherbet)".to_owned()));
		assert_eq!(song.artists, vec!["Tobokegao".to_owned()]);
		assert_eq!(song.album_artists, Vec::<String>::new());
		assert_eq!(song.album, Some("Picnic".to_owned()));
		assert_eq!(song.year, Some(2016));
		assert_eq!(song.artwork, Some(artwork_virtual_path));
	}
}
