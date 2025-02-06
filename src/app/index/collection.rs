use std::{
	borrow::BorrowMut,
	cmp::Ordering,
	collections::{HashMap, HashSet},
	path::PathBuf,
};

use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use rayon::slice::ParallelSliceMut;
use serde::{Deserialize, Serialize};
use tinyvec::TinyVec;
use unicase::UniCase;

use crate::app::index::dictionary::Dictionary;
use crate::app::index::storage::{self, AlbumKey, ArtistKey, GenreKey, SongKey};

use super::{dictionary, storage::fetch_song};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct GenreHeader {
	pub name: String,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Genre {
	pub header: GenreHeader,
	pub albums: Vec<AlbumHeader>,
	pub artists: Vec<ArtistHeader>,
	pub related_genres: HashMap<String, u32>,
	pub songs: Vec<Song>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ArtistHeader {
	pub name: UniCase<String>,
	pub num_albums_as_performer: u32,
	pub num_albums_as_additional_performer: u32,
	pub num_albums_as_composer: u32,
	pub num_albums_as_lyricist: u32,
	pub num_songs_by_genre: HashMap<String, u32>,
	pub num_songs: u32,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Artist {
	pub header: ArtistHeader,
	pub albums: Vec<Album>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct AlbumHeader {
	pub name: String,
	pub artwork: Option<PathBuf>,
	pub artists: Vec<String>,
	pub year: Option<i64>,
	pub date_added: i64,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Album {
	pub header: AlbumHeader,
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
	genres: HashMap<GenreKey, storage::Genre>,
	songs: HashMap<SongKey, storage::Song>,
	recent_albums: Vec<AlbumKey>,
}

impl Collection {
	pub fn get_albums(&self, dictionary: &Dictionary) -> Vec<AlbumHeader> {
		let mut albums = self
			.albums
			.values()
			.map(|a| make_album_header(a, dictionary))
			.collect::<Vec<_>>();
		let collator = dictionary::make_collator();
		albums.sort_by(|a, b| collator.compare(&a.name, &b.name));
		albums
	}

	pub fn get_artists(&self, dictionary: &Dictionary) -> Vec<ArtistHeader> {
		let exceptions = [dictionary.get("Various Artists"), dictionary.get("VA")];
		let mut artists = self
			.artists
			.values()
			.filter(|a| !exceptions.contains(&Some(a.name)))
			.map(|a| make_artist_header(a, dictionary))
			.collect::<Vec<_>>();
		let collator = dictionary::make_collator();
		artists.sort_by(|a, b| collator.compare(&a.name, &b.name));
		artists
	}

	pub fn get_artist(&self, dictionary: &Dictionary, artist_key: ArtistKey) -> Option<Artist> {
		let collator = dictionary::make_collator();
		self.artists.get(&artist_key).map(|artist| {
			let header = make_artist_header(artist, dictionary);
			let albums = {
				let mut albums = artist
					.all_albums
					.iter()
					.filter_map(|key| self.get_album(dictionary, key.clone()))
					.collect::<Vec<_>>();
				albums.sort_by(|a, b| match a.header.year.cmp(&b.header.year) {
					Ordering::Equal => collator.compare(&a.header.name, &b.header.name),
					o => o,
				});
				albums
			};
			Artist { header, albums }
		})
	}

	pub fn get_album(&self, dictionary: &Dictionary, album_key: AlbumKey) -> Option<Album> {
		self.albums.get(&album_key).map(|a| {
			let mut songs = a
				.songs
				.iter()
				.filter_map(|s| {
					self.get_song(
						dictionary,
						SongKey {
							virtual_path: s.virtual_path,
						},
					)
				})
				.collect::<Vec<_>>();

			songs.sort_by_key(|s| (s.disc_number.unwrap_or(-1), s.track_number.unwrap_or(-1)));

			Album {
				header: make_album_header(a, dictionary),
				songs,
			}
		})
	}

	pub fn get_random_albums(
		&self,
		dictionary: &Dictionary,
		seed: Option<u64>,
		offset: usize,
		count: usize,
	) -> Vec<Album> {
		let shuffled = {
			let mut rng = match seed {
				Some(seed) => StdRng::seed_from_u64(seed),
				None => StdRng::from_entropy(),
			};
			let mut s = self.albums.keys().collect::<Vec<_>>();
			s.shuffle(&mut rng);
			s
		};

		shuffled
			.into_iter()
			.skip(offset)
			.take(count)
			.filter_map(|k| self.get_album(dictionary, k.clone()))
			.collect()
	}

	pub fn get_recent_albums(
		&self,
		dictionary: &Dictionary,
		offset: usize,
		count: usize,
	) -> Vec<Album> {
		self.recent_albums
			.iter()
			.skip(offset)
			.take(count)
			.filter_map(|k| self.get_album(dictionary, k.clone()))
			.collect()
	}

	pub fn get_genres(&self, dictionary: &Dictionary) -> Vec<GenreHeader> {
		let mut genres = self
			.genres
			.values()
			.filter(|g| !g.albums.is_empty())
			.map(|g| make_genre_header(g, dictionary))
			.collect::<Vec<_>>();
		let collator = dictionary::make_collator();
		genres.sort_by(|a, b| collator.compare(&a.name, &b.name));
		genres
	}

	pub fn get_genre(&self, dictionary: &Dictionary, genre_key: GenreKey) -> Option<Genre> {
		self.genres.get(&genre_key).map(|genre| {
			let collator = dictionary::make_collator();

			let mut albums = genre
				.albums
				.iter()
				.filter_map(|album_key| {
					self.albums
						.get(album_key)
						.map(|a| make_album_header(a, dictionary))
				})
				.collect::<Vec<_>>();
			albums.sort_by(|a, b| collator.compare(&a.name, &b.name));

			let mut artists = genre
				.artists
				.iter()
				.filter_map(|artist_key| {
					self.artists
						.get(artist_key)
						.map(|a| make_artist_header(a, dictionary))
				})
				.collect::<Vec<_>>();
			artists.sort_by(|a, b| collator.compare(&a.name, &b.name));

			let mut songs = genre.songs.to_vec();
			self.sort_songs(&mut songs, dictionary);
			let songs = songs
				.into_iter()
				.filter_map(|k| self.get_song(dictionary, k))
				.collect::<Vec<_>>();

			let related_genres = genre
				.related_genres
				.iter()
				.map(|(genre_key, song_count)| {
					(dictionary.resolve(&genre_key.0).to_owned(), *song_count)
				})
				.collect();

			Genre {
				header: make_genre_header(genre, dictionary),
				albums,
				artists,
				related_genres,
				songs,
			}
		})
	}

	pub fn num_songs(&self) -> usize {
		self.songs.len()
	}

	pub fn get_song(&self, dictionary: &Dictionary, song_key: SongKey) -> Option<Song> {
		self.songs.get(&song_key).map(|s| fetch_song(dictionary, s))
	}

	pub fn sort_songs(&self, songs: &mut [SongKey], dictionary: &Dictionary) {
		songs.par_sort_unstable_by(|a, b| self.compare_songs(*a, *b, dictionary));
	}

	fn compare_songs(&self, a: SongKey, b: SongKey, dictionary: &Dictionary) -> Ordering {
		let (a, b) = match (self.songs.get(&a), self.songs.get(&b)) {
			(None, None) => return Ordering::Equal,
			(None, Some(_)) => return Ordering::Less,
			(Some(_), None) => return Ordering::Greater,
			(Some(a), Some(b)) => (a, b),
		};

		let a_artists = if a.album_artists.is_empty() {
			&a.artists
		} else {
			&a.album_artists
		};

		let b_artists = if b.album_artists.is_empty() {
			&b.artists
		} else {
			&b.album_artists
		};

		for (a_artist, b_artist) in a_artists.iter().zip(b_artists) {
			match dictionary.cmp(&a_artist.0, &b_artist.0) {
				Ordering::Equal => (),
				o => return o,
			}
		}

		match a_artists.len().cmp(&b_artists.len()) {
			Ordering::Equal => (),
			o => return o,
		}

		match a.year.cmp(&b.year) {
			Ordering::Equal => (),
			o => return o,
		}

		match (a.album, b.album) {
			(None, None) => (),
			(None, Some(_)) => return Ordering::Less,
			(Some(_), None) => return Ordering::Greater,
			(Some(a_album), Some(b_album)) if a_album == b_album => (),
			(Some(a_album), Some(b_album)) => return dictionary.cmp(&a_album, &b_album),
		}

		let a_key = (a.disc_number, a.track_number);
		let b_key = (b.disc_number, b.track_number);

		a_key.cmp(&b_key)
	}
}

fn make_album_header(album: &storage::Album, dictionary: &Dictionary) -> AlbumHeader {
	AlbumHeader {
		name: dictionary.resolve(&album.name).to_string(),
		artwork: album
			.artwork
			.as_ref()
			.map(|a| dictionary.resolve(&a.0))
			.map(PathBuf::from),
		artists: album
			.artists
			.iter()
			.map(|a| dictionary.resolve(&a.0).to_string())
			.collect(),
		year: album.year,
		date_added: album.date_added,
	}
}

fn make_artist_header(artist: &storage::Artist, dictionary: &Dictionary) -> ArtistHeader {
	ArtistHeader {
		name: UniCase::new(dictionary.resolve(&artist.name).to_owned()),
		num_albums_as_performer: artist.albums_as_performer.len() as u32,
		num_albums_as_additional_performer: artist.albums_as_additional_performer.len() as u32,
		num_albums_as_composer: artist.albums_as_composer.len() as u32,
		num_albums_as_lyricist: artist.albums_as_lyricist.len() as u32,
		num_songs_by_genre: artist
			.num_songs_by_genre
			.iter()
			.map(|(genre, num)| (dictionary.resolve(genre).to_string(), *num))
			.collect(),
		num_songs: artist.num_songs,
	}
}

fn make_genre_header(genre: &storage::Genre, dictionary: &Dictionary) -> GenreHeader {
	GenreHeader {
		name: dictionary.resolve(&genre.name).to_string(),
	}
}

#[derive(Clone, Default)]
pub struct Builder {
	artists: HashMap<ArtistKey, storage::Artist>,
	albums: HashMap<AlbumKey, storage::Album>,
	genres: HashMap<GenreKey, storage::Genre>,
	songs: HashMap<SongKey, storage::Song>,
}

impl Builder {
	pub fn add_song(&mut self, song: &storage::Song) {
		self.add_song_to_album(song);
		self.add_song_to_artists(song);
		self.add_song_to_genres(song);

		self.songs.insert(
			SongKey {
				virtual_path: song.virtual_path,
			},
			song.clone(),
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
			genres: self.genres,
			songs: self.songs,
			recent_albums,
		}
	}

	fn add_song_to_artists(&mut self, song: &storage::Song) {
		let album_key = song.album_key();

		let mut all_artists = TinyVec::<[ArtistKey; 8]>::new();

		for artist_key in &song.album_artists {
			all_artists.push(*artist_key);
			if let Some(album_key) = &album_key {
				let artist = self.get_or_create_artist(*artist_key);
				artist.albums_as_performer.insert(album_key.clone());
			}
		}

		for artist_key in &song.composers {
			all_artists.push(*artist_key);
			if let Some(album_key) = &album_key {
				let artist = self.get_or_create_artist(*artist_key);
				artist.albums_as_composer.insert(album_key.clone());
			}
		}

		for artist_key in &song.lyricists {
			all_artists.push(*artist_key);
			if let Some(album_key) = &album_key {
				let artist = self.get_or_create_artist(*artist_key);
				artist.albums_as_lyricist.insert(album_key.clone());
			}
		}

		for artist_key in &song.artists {
			all_artists.push(*artist_key);
			if let Some(album_key) = &album_key {
				let artist = self.get_or_create_artist(*artist_key);
				if song.album_artists.is_empty() {
					artist.albums_as_performer.insert(album_key.clone());
				} else if !song.album_artists.contains(artist_key) {
					artist
						.albums_as_additional_performer
						.insert(album_key.clone());
				}
			}
		}

		for artist_key in all_artists {
			let artist = self.get_or_create_artist(artist_key);
			artist.num_songs += 1;
			if let Some(album_key) = &album_key {
				artist.all_albums.insert(album_key.clone());
			}
			for genre in &song.genres {
				*artist
					.num_songs_by_genre
					.entry(*genre)
					.or_default()
					.borrow_mut() += 1;
			}
		}
	}

	fn get_or_create_artist(&mut self, artist_key: ArtistKey) -> &mut storage::Artist {
		self.artists
			.entry(artist_key)
			.or_insert_with(|| storage::Artist {
				name: artist_key.0,
				all_albums: HashSet::new(),
				albums_as_performer: HashSet::new(),
				albums_as_additional_performer: HashSet::new(),
				albums_as_composer: HashSet::new(),
				albums_as_lyricist: HashSet::new(),
				num_songs_by_genre: HashMap::new(),
				num_songs: 0,
			})
			.borrow_mut()
	}

	fn add_song_to_album(&mut self, song: &storage::Song) {
		let Some(album_key) = song.album_key() else {
			return;
		};

		let name = album_key.name;
		let album = self.albums.entry(album_key).or_default().borrow_mut();
		album.name = name;

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

	fn add_song_to_genres(&mut self, song: &storage::Song) {
		for name in &song.genres {
			let genre_key = GenreKey(*name);
			let genre = self.genres.entry(genre_key).or_insert(storage::Genre {
				name: *name,
				albums: HashSet::new(),
				artists: HashSet::new(),
				related_genres: HashMap::new(),
				songs: Vec::new(),
			});

			if let Some(album_key) = song.album_key() {
				genre.albums.insert(album_key);
			}

			for artist_key in &song.album_artists {
				genre.artists.insert(*artist_key);
			}

			for artist_key in &song.artists {
				genre.artists.insert(*artist_key);
			}

			for artist_key in &song.composers {
				genre.artists.insert(*artist_key);
			}

			for artist_key in &song.lyricists {
				genre.artists.insert(*artist_key);
			}

			genre.songs.push(SongKey {
				virtual_path: song.virtual_path,
			});
		}

		let genres = song.genres.clone();
		for genre in &genres {
			for other_genre in &genres {
				if genre == other_genre {
					continue;
				}
				let Some(genre) = self.genres.get_mut(&GenreKey(*genre)) else {
					continue;
				};
				genre
					.related_genres
					.entry(GenreKey(*other_genre))
					.and_modify(|n| *n += 1)
					.or_insert(1);
			}
		}
	}
}

#[cfg(test)]
mod test {

	use tinyvec::tiny_vec;

	use crate::app::{index::dictionary, scanner};
	use storage::{store_song, InternPath};

	use super::*;

	fn setup_test(songs: Vec<scanner::Song>) -> (Collection, Dictionary) {
		let mut dictionary_builder = dictionary::Builder::default();
		let mut builder = Builder::default();

		for song in songs {
			let song = store_song(&mut dictionary_builder, &song).unwrap();
			builder.add_song(&song);
		}

		let browser = builder.build();
		let dictionary = dictionary_builder.build();

		(browser, dictionary)
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
	fn can_get_all_albums() {
		let (collection, strings) = setup_test(Vec::from([
			scanner::Song {
				album: Some("ISDN".to_owned()),
				artists: vec!["FSOL".to_owned()],
				..Default::default()
			},
			scanner::Song {
				album: Some("Elysium".to_owned()),
				artists: vec!["Stratovarius".to_owned()],
				..Default::default()
			},
			scanner::Song {
				album: Some("Lifeforms".to_owned()),
				artists: vec!["FSOL".to_owned()],
				..Default::default()
			},
		]));

		let albums = collection.get_albums(&strings);
		assert_eq!(albums.len(), 3);

		assert_eq!(
			albums.into_iter().map(|a| a.name).collect::<Vec<_>>(),
			vec![
				"Elysium".to_owned(),
				"ISDN".to_owned(),
				"Lifeforms".to_owned()
			]
		);
	}

	#[test]
	fn can_get_random_albums() {
		let (collection, strings) = setup_test(Vec::from([
			scanner::Song {
				album: Some("ISDN".to_owned()),
				artists: vec!["FSOL".to_owned()],
				..Default::default()
			},
			scanner::Song {
				album: Some("Lifeforms".to_owned()),
				artists: vec!["FSOL".to_owned()],
				..Default::default()
			},
		]));

		let albums = collection.get_random_albums(&strings, None, 0, 10);
		assert_eq!(albums.len(), 2);

		assert_eq!(
			albums
				.into_iter()
				.map(|a| a.header.name)
				.collect::<HashSet<_>>(),
			HashSet::from_iter(["ISDN".to_owned(), "Lifeforms".to_owned()])
		);
	}

	#[test]
	fn can_get_recent_albums() {
		let (collection, strings) = setup_test(Vec::from([
			scanner::Song {
				album: Some("ISDN".to_owned()),
				artists: vec!["FSOL".to_owned()],
				date_added: 2000,
				..Default::default()
			},
			scanner::Song {
				album: Some("Lifeforms".to_owned()),
				artists: vec!["FSOL".to_owned()],
				date_added: 400,
				..Default::default()
			},
		]));

		let albums = collection.get_recent_albums(&strings, 0, 10);
		assert_eq!(albums.len(), 2);

		assert_eq!(
			albums
				.into_iter()
				.map(|a| a.header.name)
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
			expect_listed: bool,
		}

		let test_cases = vec![
			// Tagged as everything
			TestCase {
				album_artists: vec![artist_name.to_string()],
				artists: vec![artist_name.to_string()],
				composers: vec![artist_name.to_string()],
				lyricists: vec![artist_name.to_string()],
				expect_listed: true,
				..Default::default()
			},
			// Only tagged as artist
			TestCase {
				artists: vec![artist_name.to_string()],
				expect_listed: true,
				..Default::default()
			},
			// Only tagged as artist w/ distinct album artist
			TestCase {
				album_artists: vec![other_artist_name.to_string()],
				artists: vec![artist_name.to_string()],
				expect_listed: true,
				..Default::default()
			},
			// Tagged as artist and within album artists
			TestCase {
				album_artists: vec![artist_name.to_string(), other_artist_name.to_string()],
				artists: vec![artist_name.to_string()],
				expect_listed: true,
				..Default::default()
			},
			// Only tagged as composer
			TestCase {
				artists: vec![other_artist_name.to_string()],
				composers: vec![artist_name.to_string()],
				expect_listed: true,
				..Default::default()
			},
			// Only tagged as lyricist
			TestCase {
				artists: vec![other_artist_name.to_string()],
				lyricists: vec![artist_name.to_string()],
				expect_listed: true,
				..Default::default()
			},
			// Not tagged as lyricist
			TestCase {
				artists: vec![other_artist_name.to_string()],
				expect_listed: false,
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

			let artists = collection.get_artists(&strings);

			if test.expect_listed {
				assert!(artists.iter().any(|a| a.name == UniCase::new(artist_name)));
			} else {
				assert!(artists.iter().all(|a| a.name != UniCase::new(artist_name)));
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

		let artist =
			collection.get_artist(&strings, ArtistKey(strings.get("Stratovarius").unwrap()));

		let names = artist
			.unwrap()
			.albums
			.into_iter()
			.map(|a| a.header.name)
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
				artists: vec!["FSOL".to_owned()],
				album: Some("Lifeforms".to_owned()),
				disc_number: Some(1),
				track_number: Some(3),
				..Default::default()
			},
			scanner::Song {
				virtual_path: album_path.join("Cascade.mp3"),
				title: Some("Cascade".to_owned()),
				artists: vec!["FSOL".to_owned()],
				album: Some("Lifeforms".to_owned()),
				disc_number: Some(1),
				track_number: Some(1),
				..Default::default()
			},
			scanner::Song {
				virtual_path: album_path.join("Domain.mp3"),
				title: Some("Domain".to_owned()),
				artists: vec!["FSOL".to_owned()],
				album: Some("Lifeforms".to_owned()),
				disc_number: Some(2),
				track_number: Some(1),
				..Default::default()
			},
			scanner::Song {
				virtual_path: album_path.join("Interstat.mp3"),
				title: Some("Interstat".to_owned()),
				artists: vec!["FSOL".to_owned()],
				album: Some("Lifeforms".to_owned()),
				disc_number: Some(2),
				track_number: Some(3),
				..Default::default()
			},
		]));

		let artist = ArtistKey(strings.get("FSOL").unwrap());
		let album = collection.get_album(
			&strings,
			AlbumKey {
				artists: tiny_vec!([ArtistKey; 4] => artist),
				name: strings.get("Lifeforms").unwrap(),
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

	#[test]
	fn can_list_genres() {
		let (collection, strings) = setup_test(Vec::from([
			scanner::Song {
				virtual_path: PathBuf::from("Kai.mp3"),
				title: Some("Kai".to_owned()),
				album: Some("ISDN".to_owned()),
				artists: vec!["FSOL".to_owned()],
				genres: vec!["Ambient".to_owned()],
				..Default::default()
			},
			scanner::Song {
				virtual_path: PathBuf::from("Fantasy.mp3"),
				title: Some("Fantasy".to_owned()),
				album: Some("Nemesis".to_owned()),
				artists: vec!["Stratovarius".to_owned()],
				genres: vec!["Metal".to_owned()],
				..Default::default()
			},
		]));

		let genres = collection
			.get_genres(&strings)
			.into_iter()
			.map(|a| a.name)
			.collect::<Vec<_>>();

		assert_eq!(genres, vec!["Ambient".to_owned(), "Metal".to_owned()]);
	}

	#[test]
	fn can_get_genre() {
		let (collection, strings) = setup_test(Vec::from([
			scanner::Song {
				virtual_path: PathBuf::from("Seasons.mp3"),
				title: Some("Seasons".to_owned()),
				artists: vec!["Dragonforce".to_owned()],
				genres: vec!["Metal".to_owned()],
				..Default::default()
			},
			scanner::Song {
				virtual_path: PathBuf::from("Fantasy.mp3"),
				title: Some("Fantasy".to_owned()),
				artists: vec!["Stratovarius".to_owned()],
				genres: vec!["Metal".to_owned(), "Power Metal".to_owned()],
				..Default::default()
			},
			scanner::Song {
				virtual_path: PathBuf::from("Arv.mp3"),
				title: Some("Arv".to_owned()),
				artists: vec!["Ásmegin".to_owned()],
				genres: vec!["Metal".to_owned()],
				..Default::default()
			},
			scanner::Song {
				virtual_path: PathBuf::from("Calcium.mp3"),
				title: Some("Calcium".to_owned()),
				genres: vec!["Electronic".to_owned()],
				..Default::default()
			},
		]));

		let genre = collection
			.get_genre(&strings, GenreKey(strings.get("Metal").unwrap()))
			.unwrap();

		assert_eq!(genre.header.name, "Metal".to_owned());
		assert_eq!(genre.artists[0].name, UniCase::new("Ásmegin"));
		assert_eq!(genre.artists[1].name, UniCase::new("Dragonforce"));
		assert_eq!(genre.artists[2].name, UniCase::new("Stratovarius"));
		assert_eq!(
			genre.related_genres,
			HashMap::from_iter([("Power Metal".to_owned(), 1)])
		);
	}
}
