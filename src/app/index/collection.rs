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

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Song {
	pub real_path: PathBuf,
	pub virtual_path: PathBuf,
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

			let sort_albums =
				|a: &Album, b: &Album| (&a.year, &a.name).partial_cmp(&(&b.year, &b.name)).unwrap();

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

		album.date_added = album.date_added.max(song.date_added);

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

	use storage::InternPath;

	use super::*;

	fn setup_test(songs: Vec<scanner::Song>) -> (Collection, RodeoReader) {
		let mut strings = Rodeo::new();
		let mut builder = Builder::default();

		for song in songs {
			builder.add_song(&mut strings, &song);
		}

		let browser = builder.build();
		let strings = strings.into_reader();

		(browser, strings)
	}

	#[tokio::test]
	async fn can_get_random_albums() {
		let (collection, strings) = setup_test(Vec::from([
			scanner::Song {
				album: Some("ISDN".to_owned()),
				..Default::default()
			},
			scanner::Song {
				album: Some("Lifeforms".to_owned()),
				..Default::default()
			},
		]));

		let albums = collection.get_random_albums(&strings, 10);
		assert_eq!(albums.len(), 2);

		assert_eq!(
			albums
				.into_iter()
				.map(|a| a.name.unwrap())
				.collect::<HashSet<_>>(),
			HashSet::from_iter(["ISDN".to_owned(), "Lifeforms".to_owned()])
		);
	}

	#[tokio::test]
	async fn can_get_recent_albums() {
		let (collection, strings) = setup_test(Vec::from([
			scanner::Song {
				album: Some("ISDN".to_owned()),
				date_added: 2000,
				..Default::default()
			},
			scanner::Song {
				album: Some("Lifeforms".to_owned()),
				date_added: 400,
				..Default::default()
			},
		]));

		let albums = collection.get_recent_albums(&strings, 10);
		assert_eq!(albums.len(), 2);

		assert_eq!(
			albums
				.into_iter()
				.map(|a| a.name.unwrap())
				.collect::<Vec<_>>(),
			vec!["ISDN".to_owned(), "Lifeforms".to_owned()]
		);
	}

	#[tokio::test]
	async fn can_get_a_song() {
		let song_path = PathBuf::from_iter(["FSOL", "ISDN", "Kai.mp3"]);
		let (collection, strings) = setup_test(Vec::from([scanner::Song {
			virtual_path: song_path.clone(),
			title: Some("Kai".to_owned()),
			album: Some("ISDN".to_owned()),
			..Default::default()
		}]));

		let song = collection.get_song(
			&strings,
			SongKey {
				virtual_path: song_path.as_path().get(&strings).unwrap(),
			},
		);

		assert_eq!(
			song,
			Some(Song {
				virtual_path: song_path,
				title: Some("Kai".to_owned()),
				album: Some("ISDN".to_owned()),
				..Default::default()
			})
		);
	}
}
