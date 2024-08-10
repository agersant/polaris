use std::{
	borrow::BorrowMut,
	collections::{HashMap, HashSet},
	path::PathBuf,
};

use lasso2::{Rodeo, RodeoReader};
use rand::{rngs::ThreadRng, seq::IteratorRandom};
use serde::{Deserialize, Serialize};
use unicase::UniCase;

use crate::app::index::storage::{self, store_song, AlbumKey, ArtistKey, SongKey};
use crate::app::scanner;

use super::storage::fetch_song;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ArtistHeader {
	pub name: Option<UniCase<String>>,
	pub num_albums: u32,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Artist {
	pub header: ArtistHeader,
	pub albums: Vec<Album>,
	pub featured_on: Vec<Album>, // Albums where this artist shows up as `artist` without being `album_artist`
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
	pub fn get_artists(&self, strings: &RodeoReader) -> Vec<ArtistHeader> {
		let mut artists = self
			.artists
			.values()
			.filter(|a| a.albums.len() > 0 || a.featured_on.len() > 1)
			.map(|a| make_artist_header(a, strings))
			.collect::<Vec<_>>();
		artists.sort_by(|a, b| a.name.cmp(&b.name));
		artists
	}

	pub fn get_artist(&self, strings: &RodeoReader, artist_key: ArtistKey) -> Option<Artist> {
		self.artists.get(&artist_key).map(|a| {
			let sort_albums = |a: &Album, b: &Album| (&a.year, &a.name).cmp(&(&b.year, &b.name));

			let albums = {
				let mut albums = a
					.albums
					.iter()
					.filter_map(|key| self.get_album(strings, key.clone()))
					.collect::<Vec<_>>();
				albums.sort_by(sort_albums);
				albums
			};

			let featured_on = {
				let mut albums = a
					.featured_on
					.iter()
					.filter_map(|key| self.get_album(strings, key.clone()))
					.collect::<Vec<_>>();
				albums.sort_by(&sort_albums);
				albums
			};

			Artist {
				header: make_artist_header(a, strings),
				albums,
				featured_on,
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

fn make_artist_header(artist: &storage::Artist, strings: &RodeoReader) -> ArtistHeader {
	ArtistHeader {
		name: artist
			.name
			.map(|n| UniCase::new(strings.resolve(&n).to_owned())),
		num_albums: artist.albums.len() as u32 + artist.featured_on.len() as u32,
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
				artist.featured_on.insert(album_key.clone());
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
				featured_on: HashSet::new(),
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
	use tinyvec::TinyVec;

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

	#[test]
	fn can_list_artists() {
		let (collection, strings) = setup_test(Vec::from([
			scanner::Song {
				virtual_path: PathBuf::from("Kai.mp3"),
				title: Some("Kai".to_owned()),
				artists: vec!["FSOL".to_owned()],
				..Default::default()
			},
			scanner::Song {
				virtual_path: PathBuf::from("Fantasy.mp3"),
				title: Some("Fantasy".to_owned()),
				artists: vec!["Stratovarius".to_owned()],
				..Default::default()
			},
		]));

		let artists = collection
			.get_artists(&strings)
			.into_iter()
			.map(|a| a.name.unwrap())
			.collect::<Vec<_>>();

		assert_eq!(
			artists,
			vec![
				UniCase::new("FSOL".to_owned()),
				UniCase::new("Stratovarius".to_owned())
			]
		);
	}

	#[test]
	fn artist_list_is_sorted() {
		let (collection, strings) = setup_test(Vec::from([
			scanner::Song {
				virtual_path: PathBuf::from("Destiny.mp3"),
				title: Some("Destiny".to_owned()),
				artists: vec!["Heavenly".to_owned()],
				..Default::default()
			},
			scanner::Song {
				virtual_path: PathBuf::from("Renegade.mp3"),
				title: Some("Renegade".to_owned()),
				artists: vec!["hammerfall".to_owned()], // Lower-case `h` to validate sorting is case-insensitive
				..Default::default()
			},
		]));

		let artists = collection
			.get_artists(&strings)
			.into_iter()
			.map(|a| a.name.unwrap())
			.collect::<Vec<_>>();

		assert_eq!(
			artists,
			vec![
				UniCase::new("hammerfall".to_owned()),
				UniCase::new("Heavenly".to_owned())
			]
		);
	}

	#[test]
	fn can_get_random_albums() {
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

	#[test]
	fn can_get_recent_albums() {
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

	#[test]
	fn albums_are_associated_with_artists() {
		let artist_name = "Bestest Artist";
		let other_artist_name = "Cool Kidz";
		let album_name = "Bestest Album";

		#[derive(Debug, Default)]
		struct TestCase {
			album_artists: Vec<String>,
			artists: Vec<String>,
			composers: Vec<String>,
			lyricists: Vec<String>,
			expect_unlisted: bool,
			expect_albums: bool,
			expect_featured_on: bool,
		}

		let test_cases = vec![
			// Tagged as everything
			TestCase {
				album_artists: vec![artist_name.to_string()],
				artists: vec![artist_name.to_string()],
				composers: vec![artist_name.to_string()],
				lyricists: vec![artist_name.to_string()],
				expect_albums: true,
				..Default::default()
			},
			// Only tagged as artist
			TestCase {
				artists: vec![artist_name.to_string()],
				expect_albums: true,
				..Default::default()
			},
			// Only tagged as artist w/ distinct album artist
			TestCase {
				album_artists: vec![other_artist_name.to_string()],
				artists: vec![artist_name.to_string()],
				expect_featured_on: true,
				..Default::default()
			},
			// Tagged as artist and within album artists
			TestCase {
				album_artists: vec![artist_name.to_string(), other_artist_name.to_string()],
				artists: vec![artist_name.to_string()],
				expect_albums: true,
				..Default::default()
			},
			// Only tagged as composer
			TestCase {
				artists: vec![other_artist_name.to_string()],
				composers: vec![artist_name.to_string()],
				expect_unlisted: true,
				..Default::default()
			},
			// Only tagged as lyricist
			TestCase {
				artists: vec![other_artist_name.to_string()],
				lyricists: vec![artist_name.to_string()],
				expect_unlisted: true,
				..Default::default()
			},
		];

		for test in test_cases {
			let (collection, strings) = setup_test(Vec::from([scanner::Song {
				virtual_path: PathBuf::from_iter(["Some Directory", "Cool Song.mp3"]),
				album: Some(album_name.to_owned()),
				album_artists: test.album_artists.clone(),
				artists: test.artists.clone(),
				composers: test.composers.clone(),
				lyricists: test.lyricists.clone(),
				..Default::default()
			}]));

			let artist_key = ArtistKey {
				name: strings.get(artist_name),
			};
			let artist = collection.get_artist(&strings, artist_key);

			let artist = match test.expect_unlisted {
				true => {
					assert!(artist.is_none());
					continue;
				}
				false => artist.unwrap(),
			};

			let names = |a: &Vec<Album>| {
				a.iter()
					.map(|a| a.name.to_owned().unwrap())
					.collect::<Vec<_>>()
			};

			if test.expect_albums {
				assert_eq!(names(&artist.albums), vec![album_name]);
			} else {
				assert!(names(&artist.albums).is_empty());
			}

			if test.expect_featured_on {
				assert_eq!(names(&artist.featured_on), vec![album_name]);
			} else {
				assert!(names(&artist.featured_on).is_empty());
			}
		}
	}

	#[test]
	fn albums_are_sorted_by_year() {
		let (collection, strings) = setup_test(Vec::from([
			scanner::Song {
				virtual_path: PathBuf::from("Rebel.mp3"),
				title: Some("Rebel".to_owned()),
				album: Some("Destiny".to_owned()),
				artists: vec!["Stratovarius".to_owned()],
				year: Some(1998),
				..Default::default()
			},
			scanner::Song {
				virtual_path: PathBuf::from("Eternity.mp3"),
				title: Some("Eternity".to_owned()),
				album: Some("Episode".to_owned()),
				artists: vec!["Stratovarius".to_owned()],
				year: Some(1996),
				..Default::default()
			},
			scanner::Song {
				virtual_path: PathBuf::from("Broken.mp3"),
				title: Some("Broken".to_owned()),
				album: Some("Survive".to_owned()),
				artists: vec!["Stratovarius".to_owned()],
				year: Some(2022),
				..Default::default()
			},
		]));

		let artist = collection.get_artist(
			&strings,
			ArtistKey {
				name: strings.get("Stratovarius"),
			},
		);

		let names = artist
			.unwrap()
			.albums
			.into_iter()
			.map(|a| a.name.unwrap())
			.collect::<Vec<_>>();

		assert_eq!(
			names,
			vec![
				"Episode".to_owned(),
				"Destiny".to_owned(),
				"Survive".to_owned(),
			]
		);
	}

	#[test]
	fn album_songs_are_sorted() {
		let album_path = PathBuf::from_iter(["FSOL", "Lifeforms"]);
		let (collection, strings) = setup_test(Vec::from([
			scanner::Song {
				virtual_path: album_path.join("Flak.mp3"),
				title: Some("Flak".to_owned()),
				album: Some("Lifeforms".to_owned()),
				disc_number: Some(1),
				track_number: Some(3),
				..Default::default()
			},
			scanner::Song {
				virtual_path: album_path.join("Cascade.mp3"),
				title: Some("Cascade".to_owned()),
				album: Some("Lifeforms".to_owned()),
				disc_number: Some(1),
				track_number: Some(1),
				..Default::default()
			},
			scanner::Song {
				virtual_path: album_path.join("Domain.mp3"),
				title: Some("Domain".to_owned()),
				album: Some("Lifeforms".to_owned()),
				disc_number: Some(2),
				track_number: Some(1),
				..Default::default()
			},
			scanner::Song {
				virtual_path: album_path.join("Interstat.mp3"),
				title: Some("Interstat".to_owned()),
				album: Some("Lifeforms".to_owned()),
				disc_number: Some(2),
				track_number: Some(3),
				..Default::default()
			},
		]));

		let album = collection.get_album(
			&strings,
			AlbumKey {
				artists: TinyVec::new(),
				name: strings.get("Lifeforms"),
			},
		);

		let titles = album
			.unwrap()
			.songs
			.into_iter()
			.map(|s| s.title.unwrap())
			.collect::<Vec<_>>();

		assert_eq!(
			titles,
			vec![
				"Cascade".to_owned(),
				"Flak".to_owned(),
				"Domain".to_owned(),
				"Interstat".to_owned(),
			]
		);
	}

	#[test]
	fn can_get_a_song() {
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
