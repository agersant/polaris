use std::{
	borrow::BorrowMut,
	collections::{HashMap, HashSet},
	path::PathBuf,
};

use lasso2::{Rodeo, RodeoReader, Spur};
use rand::{rngs::ThreadRng, seq::IteratorRandom};
use serde::{Deserialize, Serialize};
use tinyvec::TinyVec;
use unicase::UniCase;

use crate::app::index::storage::{self, store_song, AlbumKey, ArtistKey, SongKey};
use crate::app::scanner;

use super::storage::fetch_song;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ArtistHeader {
	pub name: UniCase<String>,
	pub num_albums_as_performer: u32,
	pub num_albums_as_additional_performer: u32,
	pub num_albums_as_composer: u32,
	pub num_albums_as_lyricist: u32,
	pub num_songs_by_genre: HashMap<String, u32>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Artist {
	pub header: ArtistHeader,
	pub albums_as_performer: Vec<Album>,
	pub albums_as_additional_performer: Vec<Album>, // Albums where this artist shows up as `artist` without being `album_artist`
	pub albums_as_composer: Vec<Album>,
	pub albums_as_lyricist: Vec<Album>,
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
		let exceptions = vec![strings.get("Various Artists"), strings.get("VA")];
		let mut artists = self
			.artists
			.values()
			.filter(|a| !exceptions.contains(&Some(a.name)))
			.map(|a| make_artist_header(a, strings))
			.collect::<Vec<_>>();
		artists.sort_by(|a, b| a.name.cmp(&b.name));
		artists
	}

	pub fn get_artist(&self, strings: &RodeoReader, artist_key: ArtistKey) -> Option<Artist> {
		self.artists.get(&artist_key).map(|a| {
			let sort_albums = |a: &Album, b: &Album| (&a.year, &a.name).cmp(&(&b.year, &b.name));

			let list_albums = |keys: &HashSet<AlbumKey>| {
				let mut albums = keys
					.iter()
					.filter_map(|key| self.get_album(strings, key.clone()))
					.collect::<Vec<_>>();
				albums.sort_by(sort_albums);
				albums
			};

			let albums_as_performer = list_albums(&a.albums_as_performer);
			let albums_as_additional_performer = list_albums(&a.albums_as_additional_performer);
			let albums_as_composer = list_albums(&a.albums_as_composer);
			let albums_as_lyricist = list_albums(&a.albums_as_lyricist);

			Artist {
				header: make_artist_header(a, strings),
				albums_as_performer,
				albums_as_additional_performer,
				albums_as_composer,
				albums_as_lyricist,
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
		name: UniCase::new(strings.resolve(&artist.name).to_owned()),
		num_albums_as_performer: artist.albums_as_performer.len() as u32,
		num_albums_as_additional_performer: artist.albums_as_additional_performer.len() as u32,
		num_albums_as_composer: artist.albums_as_composer.len() as u32,
		num_albums_as_lyricist: artist.albums_as_lyricist.len() as u32,
		num_songs_by_genre: artist
			.num_songs_by_genre
			.iter()
			.map(|(genre, num)| (strings.resolve(genre).to_string(), *num))
			.collect(),
	}
}

#[derive(Default)]
pub struct Builder {
	artists: HashMap<ArtistKey, storage::Artist>,
	albums: HashMap<AlbumKey, storage::Album>,
	songs: HashMap<SongKey, storage::Song>,
}

impl Builder {
	pub fn add_song(
		&mut self,
		strings: &mut Rodeo,
		minuscules: &mut HashMap<String, Spur>,
		song: &scanner::Song,
	) {
		let Some(song) = store_song(strings, minuscules, song) else {
			return;
		};

		self.add_song_to_album(&song);
		self.add_song_to_artists(&song);

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

	fn add_song_to_artists(&mut self, song: &storage::Song) {
		let album_key: AlbumKey = song.album_key();

		let mut all_artists = TinyVec::<[Spur; 8]>::new();

		for name in &song.album_artists {
			let artist = self.get_or_create_artist(*name);
			artist.albums_as_performer.insert(album_key.clone());
			all_artists.push(*name);
		}

		for name in &song.composers {
			let artist = self.get_or_create_artist(*name);
			artist.albums_as_composer.insert(album_key.clone());
			all_artists.push(*name);
		}

		for name in &song.lyricists {
			let artist = self.get_or_create_artist(*name);
			artist.albums_as_lyricist.insert(album_key.clone());
			all_artists.push(*name);
		}

		for name in &song.artists {
			let artist = self.get_or_create_artist(*name);
			all_artists.push(*name);
			if song.album_artists.is_empty() {
				artist.albums_as_performer.insert(album_key.clone());
			} else if !song.album_artists.contains(name) {
				artist
					.albums_as_additional_performer
					.insert(album_key.clone());
			}
		}

		for name in all_artists {
			let artist = self.get_or_create_artist(name);
			for genre in &song.genres {
				*artist
					.num_songs_by_genre
					.entry(*genre)
					.or_default()
					.borrow_mut() += 1;
			}
		}
	}

	fn get_or_create_artist(&mut self, name: lasso2::Spur) -> &mut storage::Artist {
		let artist_key = ArtistKey { name: Some(name) };
		self.artists
			.entry(artist_key)
			.or_insert_with(|| storage::Artist {
				name,
				albums_as_performer: HashSet::new(),
				albums_as_additional_performer: HashSet::new(),
				albums_as_composer: HashSet::new(),
				albums_as_lyricist: HashSet::new(),
				num_songs_by_genre: HashMap::new(),
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
		let mut minuscules = HashMap::new();
		let mut builder = Builder::default();

		for song in songs {
			builder.add_song(&mut strings, &mut minuscules, &song);
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
			.map(|a| a.name)
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
			.map(|a| a.name)
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
	fn artists_with_diverging_case_are_merged() {
		let (collection, strings) = setup_test(Vec::from([
			scanner::Song {
				virtual_path: PathBuf::from("Rain of Fury.mp3"),
				title: Some("Rain of Fury".to_owned()),
				artists: vec!["Rhapsody Of Fire".to_owned()],
				..Default::default()
			},
			scanner::Song {
				virtual_path: PathBuf::from("Emerald Sword.mp3"),
				title: Some("Chains of Destiny".to_owned()),
				artists: vec!["Rhapsody of Fire".to_owned()],
				..Default::default()
			},
		]));

		let artists = collection
			.get_artists(&strings)
			.into_iter()
			.map(|a| a.name)
			.collect::<Vec<_>>();

		assert_eq!(artists, vec![UniCase::new("Rhapsody of Fire".to_owned()),]);
	}

	#[test]
	fn artists_list_excludes_various_artists() {
		let (collection, strings) = setup_test(Vec::from([
			scanner::Song {
				virtual_path: PathBuf::from("Rain of Fury.mp3"),
				title: Some("Rain of Fury".to_owned()),
				artists: vec!["Rhapsody Of Fire".to_owned()],
				album_artists: vec!["Various Artists".to_owned()],
				..Default::default()
			},
			scanner::Song {
				virtual_path: PathBuf::from("Paradise.mp3"),
				title: Some("Paradise".to_owned()),
				artists: vec!["Stratovarius".to_owned()],
				album_artists: vec!["Various Artists".to_owned()],
				..Default::default()
			},
		]));

		let artists = collection
			.get_artists(&strings)
			.into_iter()
			.map(|a| a.name)
			.collect::<Vec<_>>();

		assert_eq!(
			artists,
			vec![
				UniCase::new("Rhapsody of Fire".to_owned()),
				UniCase::new("Stratovarius".to_owned()),
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
			expect_performer: bool,
			expect_additional_performer: bool,
			expect_composer: bool,
			expect_lyricist: bool,
		}

		let test_cases = vec![
			// Tagged as everything
			TestCase {
				album_artists: vec![artist_name.to_string()],
				artists: vec![artist_name.to_string()],
				composers: vec![artist_name.to_string()],
				lyricists: vec![artist_name.to_string()],
				expect_performer: true,
				expect_composer: true,
				expect_lyricist: true,
				..Default::default()
			},
			// Only tagged as artist
			TestCase {
				artists: vec![artist_name.to_string()],
				expect_performer: true,
				..Default::default()
			},
			// Only tagged as artist w/ distinct album artist
			TestCase {
				album_artists: vec![other_artist_name.to_string()],
				artists: vec![artist_name.to_string()],
				expect_additional_performer: true,
				..Default::default()
			},
			// Tagged as artist and within album artists
			TestCase {
				album_artists: vec![artist_name.to_string(), other_artist_name.to_string()],
				artists: vec![artist_name.to_string()],
				expect_performer: true,
				..Default::default()
			},
			// Only tagged as composer
			TestCase {
				artists: vec![other_artist_name.to_string()],
				composers: vec![artist_name.to_string()],
				expect_composer: true,
				..Default::default()
			},
			// Only tagged as lyricist
			TestCase {
				artists: vec![other_artist_name.to_string()],
				lyricists: vec![artist_name.to_string()],
				expect_lyricist: true,
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
			let artist = collection.get_artist(&strings, artist_key).unwrap();

			let names = |a: &Vec<Album>| {
				a.iter()
					.map(|a| a.name.to_owned().unwrap())
					.collect::<Vec<_>>()
			};

			if test.expect_performer {
				assert_eq!(names(&artist.albums_as_performer), vec![album_name]);
			} else {
				assert!(names(&artist.albums_as_performer).is_empty());
			}

			if test.expect_additional_performer {
				assert_eq!(
					names(&artist.albums_as_additional_performer),
					vec![album_name]
				);
			} else {
				assert!(names(&artist.albums_as_additional_performer).is_empty());
			}

			if test.expect_composer {
				assert_eq!(names(&artist.albums_as_composer), vec![album_name]);
			} else {
				assert!(names(&artist.albums_as_composer).is_empty());
			}

			if test.expect_lyricist {
				assert_eq!(names(&artist.albums_as_lyricist), vec![album_name]);
			} else {
				assert!(names(&artist.albums_as_lyricist).is_empty());
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
			.albums_as_performer
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
