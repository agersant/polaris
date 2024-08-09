use std::{
	borrow::BorrowMut,
	collections::{HashMap, HashSet},
	path::PathBuf,
};

use lasso2::{Rodeo, RodeoReader};
use rand::{rngs::ThreadRng, seq::IteratorRandom};
use serde::{Deserialize, Serialize};

use crate::app::index::storage::{self, store_song, AlbumKey, ArtistKey, SongKey};
use crate::app::scanner;

use super::storage::fetch_song;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Artist {
	pub name: Option<String>,
	pub albums: Vec<Album>,
	pub song_credits: Vec<Album>, // Albums where this artist shows up as `artist` without being `album_artist`
	pub composer_credits: Vec<Album>, // Albums where this artist shows up as `composer` without being `artist` or `album_artist`
	pub lyricist_credits: Vec<Album>, // Albums where this artist shows up as `lyricist` without being `artist` or `album_artist`
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
	pub fn get_artist(&self, strings: &RodeoReader, artist_key: ArtistKey) -> Option<Artist> {
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

			let sort_albums = |a: &Album, b: &Album| {
				(&a.artists, a.year, &a.name)
					.partial_cmp(&(&b.artists, b.year, &b.name))
					.unwrap()
			};

			let song_credits = {
				let mut albums = a
					.song_credits
					.iter()
					.filter_map(|key| self.get_album(strings, key.clone()))
					.collect::<Vec<_>>();
				albums.sort_by(&sort_albums);
				albums
			};

			let composer_credits = {
				let mut albums = a
					.composer_credits
					.iter()
					.filter_map(|key| self.get_album(strings, key.clone()))
					.collect::<Vec<_>>();
				albums.sort_by(&sort_albums);
				albums
			};

			let lyricist_credits = {
				let mut albums = a
					.lyricist_credits
					.iter()
					.filter_map(|key| self.get_album(strings, key.clone()))
					.collect::<Vec<_>>();
				albums.sort_by(&sort_albums);
				albums
			};

			Artist {
				name: a.name.map(|s| strings.resolve(&s).to_string()),
				albums,
				song_credits,
				composer_credits,
				lyricist_credits,
			}
		})
	}

	pub fn get_album(&self, strings: &RodeoReader, album_key: AlbumKey) -> Option<Album> {
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

	pub fn get_random_albums(&self, strings: &RodeoReader, count: usize) -> Vec<Album> {
		self.albums
			.keys()
			.choose_multiple(&mut ThreadRng::default(), count)
			.into_iter()
			.filter_map(|k| self.get_album(strings, k.clone()))
			.collect()
	}

	pub fn get_recent_albums(&self, strings: &RodeoReader, count: usize) -> Vec<Album> {
		self.recent_albums
			.iter()
			.take(count)
			.filter_map(|k| self.get_album(strings, k.clone()))
			.collect()
	}

	pub fn get_song(&self, strings: &RodeoReader, song_key: SongKey) -> Option<Song> {
		self.songs.get(&song_key).map(|s| fetch_song(strings, s))
	}
}

#[derive(Default)]
pub struct Builder {
	artists: HashMap<ArtistKey, storage::Artist>,
	albums: HashMap<AlbumKey, storage::Album>,
	songs: HashMap<SongKey, storage::Song>,
}

impl Builder {
	pub fn add_song(&mut self, strings: &mut Rodeo, song: &scanner::Song) {
		let Some(song) = store_song(strings, song) else {
			return;
		};

		self.add_song_to_album(&song);
		self.add_album_to_artists(&song);

		self.songs.insert(
			SongKey {
				virtual_path: song.virtual_path,
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

		for name in &song.album_artists {
			let artist = self.get_or_create_artist(*name);
			artist.albums.insert(album_key.clone());
		}

		for name in &song.artists {
			let artist = self.get_or_create_artist(*name);
			if song.album_artists.is_empty() {
				artist.albums.insert(album_key.clone());
			} else if !song.album_artists.contains(name) {
				artist.song_credits.insert(album_key.clone());
			}
		}

		for name in &song.composers {
			let is_also_artist = song.artists.contains(name);
			let is_also_album_artist = song.artists.contains(name);
			if !is_also_artist && !is_also_album_artist {
				let artist = self.get_or_create_artist(*name);
				artist.composer_credits.insert(album_key.clone());
			}
		}

		for name in &song.lyricists {
			let is_also_artist = song.artists.contains(name);
			let is_also_album_artist = song.artists.contains(name);
			if !is_also_artist && !is_also_album_artist {
				let artist = self.get_or_create_artist(*name);
				artist.lyricist_credits.insert(album_key.clone());
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
				song_credits: HashSet::new(),
				composer_credits: HashSet::new(),
				lyricist_credits: HashSet::new(),
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
