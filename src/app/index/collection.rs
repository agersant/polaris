use std::{
	borrow::BorrowMut,
	collections::{HashMap, HashSet},
	hash::Hash,
	path::PathBuf,
};

use lasso2::ThreadedRodeo;
use rand::{rngs::ThreadRng, seq::IteratorRandom};
use serde::{Deserialize, Serialize};
use tinyvec::TinyVec;

use crate::app::index::{InternPath, PathID};
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
	artists: HashMap<ArtistKey, storage::Artist>,
	albums: HashMap<AlbumKey, storage::Album>,
	songs: HashMap<SongKey, storage::Song>,
	recent_albums: Vec<AlbumKey>,
}

impl Collection {
	pub fn get_artist(&self, strings: &ThreadedRodeo, artist_key: ArtistKey) -> Option<Artist> {
		self.artists.get(&artist_key).map(|a| {
			let albums = {
				let mut albums = a
					.albums
					.iter()
					.filter_map(|key| self.get_album(strings, key.clone()))
					.collect::<Vec<_>>();
				albums.sort_by(|a, b| (a.year, &a.name).partial_cmp(&(b.year, &b.name)).unwrap());
				albums
			};

			let album_appearances = {
				let mut album_appearances = a
					.album_appearances
					.iter()
					.filter_map(|key| self.get_album(strings, key.clone()))
					.collect::<Vec<_>>();
				album_appearances.sort_by(|a, b| {
					(&a.artists, a.year, &a.name)
						.partial_cmp(&(&b.artists, b.year, &b.name))
						.unwrap()
				});
				album_appearances
			};

			Artist {
				name: a.name.map(|s| strings.resolve(&s).to_string()),
				albums,
				album_appearances,
			}
		})
	}

	pub fn get_album(&self, strings: &ThreadedRodeo, album_key: AlbumKey) -> Option<Album> {
		self.albums.get(&album_key).map(|a| {
			let mut songs = a
				.songs
				.iter()
				.filter_map(|s| {
					self.get_song(
						strings,
						SongKey {
							virtual_path: s.virtual_path,
						},
					)
				})
				.collect::<Vec<_>>();

			songs.sort_by_key(|s| (s.disc_number.unwrap_or(-1), s.track_number.unwrap_or(-1)));

			Album {
				name: a.name.map(|s| strings.resolve(&s).to_string()),
				artwork: a
					.artwork
					.as_ref()
					.map(|a| strings.resolve(&a.0))
					.map(PathBuf::from),
				artists: a
					.artists
					.iter()
					.map(|a| strings.resolve(a).to_string())
					.collect(),
				year: a.year,
				date_added: a.date_added,
				songs,
			}
		})
	}

	pub fn get_random_albums(&self, strings: &ThreadedRodeo, count: usize) -> Vec<Album> {
		self.albums
			.keys()
			.choose_multiple(&mut ThreadRng::default(), count)
			.into_iter()
			.filter_map(|k| self.get_album(strings, k.clone()))
			.collect()
	}

	pub fn get_recent_albums(&self, strings: &ThreadedRodeo, count: usize) -> Vec<Album> {
		self.recent_albums
			.iter()
			.take(count)
			.filter_map(|k| self.get_album(strings, k.clone()))
			.collect()
	}

	pub fn get_song(&self, strings: &ThreadedRodeo, song_key: SongKey) -> Option<Song> {
		self.songs.get(&song_key).map(|s| Song {
			path: PathBuf::from(strings.resolve(&s.path.0)),
			virtual_path: PathBuf::from(strings.resolve(&s.virtual_path.0)),
			virtual_parent: PathBuf::from(strings.resolve(&s.virtual_parent.0)),
			track_number: s.track_number,
			disc_number: s.disc_number,
			title: s.title.map(|s| strings.resolve(&s).to_string()),
			artists: s
				.artists
				.iter()
				.map(|s| strings.resolve(&s).to_string())
				.collect(),
			album_artists: s
				.album_artists
				.iter()
				.map(|s| strings.resolve(&s).to_string())
				.collect(),
			year: s.year,
			album: s.album.map(|s| strings.resolve(&s).to_string()),
			artwork: s.artwork.map(|a| PathBuf::from(strings.resolve(&a.0))),
			duration: s.duration,
			lyricists: s
				.lyricists
				.iter()
				.map(|s| strings.resolve(&s).to_string())
				.collect(),
			composers: s
				.composers
				.iter()
				.map(|s| strings.resolve(&s).to_string())
				.collect(),
			genres: s
				.genres
				.iter()
				.map(|s| strings.resolve(&s).to_string())
				.collect(),
			labels: s
				.labels
				.iter()
				.map(|s| strings.resolve(&s).to_string())
				.collect(),
			date_added: s.date_added,
		})
	}
}

#[derive(Default)]
pub struct Builder {
	artists: HashMap<ArtistKey, storage::Artist>,
	albums: HashMap<AlbumKey, storage::Album>,
	songs: HashMap<SongKey, storage::Song>,
}

impl Builder {
	pub fn add_song(&mut self, strings: &mut ThreadedRodeo, song: scanner::Song) {
		let Some(path_id) = song.path.get_or_intern(strings) else {
			return;
		};

		let Some(virtual_path_id) = song.virtual_path.get_or_intern(strings) else {
			return;
		};

		let Some(virtual_parent_id) = song.virtual_parent.get_or_intern(strings) else {
			return;
		};

		let Some(artwork_id) = song.artwork.map(|s| s.get_or_intern(strings)) else {
			return;
		};

		let song = storage::Song {
			path: path_id,
			virtual_path: virtual_path_id,
			virtual_parent: virtual_parent_id,
			track_number: song.track_number,
			disc_number: song.disc_number,
			title: song.title.map(|s| strings.get_or_intern(s)),
			artists: song
				.artists
				.into_iter()
				.map(|s| strings.get_or_intern(s))
				.collect(),
			album_artists: song
				.album_artists
				.into_iter()
				.map(|s| strings.get_or_intern(s))
				.collect(),
			year: song.year,
			album: song.album.map(|s| strings.get_or_intern(s)),
			artwork: artwork_id,
			duration: song.duration,
			lyricists: song
				.lyricists
				.into_iter()
				.map(|s| strings.get_or_intern(s))
				.collect(),
			composers: song
				.composers
				.into_iter()
				.map(|s| strings.get_or_intern(s))
				.collect(),
			genres: song
				.genres
				.into_iter()
				.map(|s| strings.get_or_intern(s))
				.collect(),
			labels: song
				.labels
				.into_iter()
				.map(|s| strings.get_or_intern(s))
				.collect(),
			date_added: song.date_added,
		};

		self.add_song_to_album(&song);
		self.add_album_to_artists(&song);

		self.songs.insert(
			SongKey {
				virtual_path: virtual_path_id,
			},
			song,
		);
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

	fn add_album_to_artists(&mut self, song: &storage::Song) {
		let album_key: AlbumKey = song.album_key();

		for artist_name in &song.album_artists {
			let artist = self.get_or_create_artist(*artist_name);
			artist.albums.insert(album_key.clone());
		}

		for artist_name in &song.artists {
			let artist = self.get_or_create_artist(*artist_name);
			if song.album_artists.is_empty() {
				artist.albums.insert(album_key.clone());
			} else if !song.album_artists.contains(artist_name) {
				artist.album_appearances.insert(album_key.clone());
			}
		}
	}

	fn get_or_create_artist(&mut self, name: lasso2::Spur) -> &mut storage::Artist {
		let artist_key = ArtistKey { name: Some(name) };
		self.artists
			.entry(artist_key)
			.or_insert_with(|| storage::Artist {
				name: Some(name),
				albums: HashSet::new(),
				album_appearances: HashSet::new(),
			})
			.borrow_mut()
	}

	fn add_song_to_album(&mut self, song: &storage::Song) {
		let album_key = song.album_key();
		let album = self.albums.entry(album_key).or_default().borrow_mut();

		if album.name.is_none() {
			album.name = song.album;
		}

		if album.artwork.is_none() {
			album.artwork = song.artwork;
		}

		if album.year.is_none() {
			album.year = song.year;
		}

		album.date_added = album.date_added.min(song.date_added);

		if !song.album_artists.is_empty() {
			album.artists = song.album_artists.clone();
		} else if !song.artists.is_empty() {
			album.artists = song.artists.clone();
		}

		album.songs.insert(SongKey {
			virtual_path: song.virtual_path,
		});
	}
}

mod storage {
	use super::*;

	#[derive(Serialize, Deserialize)]
	pub struct Artist {
		pub name: Option<lasso2::Spur>,
		pub albums: HashSet<AlbumKey>,
		pub album_appearances: HashSet<AlbumKey>,
	}

	#[derive(Clone, Default, Serialize, Deserialize)]
	pub struct Album {
		pub name: Option<lasso2::Spur>,
		pub artwork: Option<PathID>,
		pub artists: Vec<lasso2::Spur>,
		pub year: Option<i64>,
		pub date_added: i64,
		pub songs: HashSet<SongKey>,
	}

	#[derive(Clone, Serialize, Deserialize)]
	pub struct Song {
		pub path: PathID,
		pub virtual_path: PathID,
		pub virtual_parent: PathID,
		pub track_number: Option<i64>,
		pub disc_number: Option<i64>,
		pub title: Option<lasso2::Spur>,
		pub artists: Vec<lasso2::Spur>,
		pub album_artists: Vec<lasso2::Spur>,
		pub year: Option<i64>,
		pub album: Option<lasso2::Spur>,
		pub artwork: Option<PathID>,
		pub duration: Option<i64>,
		pub lyricists: Vec<lasso2::Spur>,
		pub composers: Vec<lasso2::Spur>,
		pub genres: Vec<lasso2::Spur>,
		pub labels: Vec<lasso2::Spur>,
		pub date_added: i64,
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
	}
}

#[derive(Copy, Clone, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct ArtistKey {
	pub name: Option<lasso2::Spur>,
}

#[derive(Clone, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct AlbumKey {
	pub artists: TinyVec<[lasso2::Spur; 4]>,
	pub name: Option<lasso2::Spur>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct SongKey {
	pub virtual_path: PathID,
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
			.get_song(song_virtual_path.clone())
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
