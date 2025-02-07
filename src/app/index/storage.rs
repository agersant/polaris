use std::{
	collections::{HashMap, HashSet},
	path::{Path, PathBuf},
};

use lasso2::Spur;
use log::error;
use serde::{Deserialize, Serialize};
use tinyvec::TinyVec;

use crate::app::scanner;

use crate::app::index::dictionary::{self, Dictionary};

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum File {
	Directory(PathKey),
	Song(PathKey),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Genre {
	pub name: Spur,
	pub albums: HashSet<AlbumKey>,
	pub artists: HashSet<ArtistKey>,
	pub related_genres: HashMap<GenreKey, u32>,
	pub songs: Vec<SongKey>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Artist {
	pub name: Spur,
	pub all_albums: HashSet<AlbumKey>,
	pub albums_as_performer: HashSet<AlbumKey>,
	pub albums_as_additional_performer: HashSet<AlbumKey>,
	pub albums_as_composer: HashSet<AlbumKey>,
	pub albums_as_lyricist: HashSet<AlbumKey>,
	pub num_songs_by_genre: HashMap<Spur, u32>,
	pub num_songs: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Album {
	pub name: Spur,
	pub artwork: Option<PathKey>,
	pub artists: TinyVec<[ArtistKey; 1]>,
	pub year: Option<i64>,
	pub date_added: i64,
	pub songs: HashSet<SongKey>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Song {
	pub real_path: PathKey,
	pub virtual_path: PathKey,
	pub track_number: Option<i64>,
	pub disc_number: Option<i64>,
	pub title: Option<Spur>,
	pub artists: TinyVec<[ArtistKey; 1]>,
	pub album_artists: TinyVec<[ArtistKey; 1]>,
	pub year: Option<i64>,
	pub album: Option<Spur>,
	pub artwork: Option<PathKey>,
	pub duration: Option<i64>,
	pub lyricists: TinyVec<[ArtistKey; 0]>,
	pub composers: TinyVec<[ArtistKey; 0]>,
	pub genres: TinyVec<[Spur; 1]>,
	pub labels: TinyVec<[Spur; 0]>,
	pub date_added: i64,
}

#[derive(
	Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
pub struct PathKey(pub Spur);

#[derive(Copy, Clone, Debug, Default, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct GenreKey(pub Spur);

#[derive(Copy, Clone, Debug, Default, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct ArtistKey(pub Spur);

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct AlbumKey {
	pub artists: TinyVec<[ArtistKey; 4]>,
	pub name: Spur,
}

#[derive(Copy, Clone, Debug, Default, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct SongKey {
	pub virtual_path: PathKey,
}
impl nohash_hasher::IsEnabled for SongKey {}

impl Song {
	pub fn album_key(&self) -> Option<AlbumKey> {
		let main_artists = match self.album_artists.is_empty() {
			true => &self.artists,
			false => &self.album_artists,
		};

		if main_artists.is_empty() {
			return None;
		}

		self.album.map(|name| AlbumKey {
			artists: main_artists.iter().cloned().collect(),
			name,
		})
	}
}

pub fn store_song(
	dictionary_builder: &mut dictionary::Builder,
	song: &scanner::Song,
) -> Option<Song> {
	let real_path = (&song.real_path).get_or_intern(dictionary_builder)?;
	let virtual_path = (&song.virtual_path).get_or_intern(dictionary_builder)?;

	let artwork = match &song.artwork {
		Some(a) => match a.get_or_intern(dictionary_builder) {
			Some(a) => Some(a),
			None => return None,
		},
		None => None,
	};

	let mut canonicalize = |s: &String| dictionary_builder.get_or_intern_canon(s);

	Some(Song {
		real_path,
		virtual_path,
		track_number: song.track_number,
		disc_number: song.disc_number,
		title: song.title.as_ref().and_then(&mut canonicalize),
		artists: song
			.artists
			.iter()
			.filter_map(&mut canonicalize)
			.map(ArtistKey)
			.collect(),
		album_artists: song
			.album_artists
			.iter()
			.filter_map(&mut canonicalize)
			.map(ArtistKey)
			.collect(),
		year: song.year,
		album: song.album.as_ref().and_then(&mut canonicalize),
		artwork: artwork,
		duration: song.duration,
		lyricists: song
			.lyricists
			.iter()
			.filter_map(&mut canonicalize)
			.map(ArtistKey)
			.collect(),
		composers: song
			.composers
			.iter()
			.filter_map(&mut canonicalize)
			.map(ArtistKey)
			.collect(),
		genres: song.genres.iter().filter_map(&mut canonicalize).collect(),
		labels: song.labels.iter().filter_map(&mut canonicalize).collect(),
		date_added: song.date_added,
	})
}

pub fn fetch_song(dictionary: &Dictionary, song: &Song) -> super::Song {
	super::Song {
		real_path: PathBuf::from(dictionary.resolve(&song.real_path.0)),
		virtual_path: PathBuf::from(dictionary.resolve(&song.virtual_path.0)),
		track_number: song.track_number,
		disc_number: song.disc_number,
		title: song.title.map(|s| dictionary.resolve(&s).to_string()),
		artists: song
			.artists
			.iter()
			.map(|k| dictionary.resolve(&k.0).to_string())
			.collect(),
		album_artists: song
			.album_artists
			.iter()
			.map(|k| dictionary.resolve(&k.0).to_string())
			.collect(),
		year: song.year,
		album: song.album.map(|s| dictionary.resolve(&s).to_string()),
		artwork: song
			.artwork
			.map(|a| PathBuf::from(dictionary.resolve(&a.0))),
		duration: song.duration,
		lyricists: song
			.lyricists
			.iter()
			.map(|k| dictionary.resolve(&k.0).to_string())
			.collect(),
		composers: song
			.composers
			.iter()
			.map(|k| dictionary.resolve(&k.0).to_string())
			.collect(),
		genres: song
			.genres
			.iter()
			.map(|s| dictionary.resolve(s).to_string())
			.collect(),
		labels: song
			.labels
			.iter()
			.map(|s| dictionary.resolve(s).to_string())
			.collect(),
		date_added: song.date_added,
	}
}

pub trait InternPath {
	fn get_or_intern(self, dictionary: &mut dictionary::Builder) -> Option<PathKey>;
	fn get(self, dictionary: &Dictionary) -> Option<PathKey>;
}

impl<P: AsRef<Path>> InternPath for P {
	fn get_or_intern(self, dictionary: &mut dictionary::Builder) -> Option<PathKey> {
		let id = self
			.as_ref()
			.as_os_str()
			.to_str()
			.map(|s| dictionary.get_or_intern(s))
			.map(PathKey);
		if id.is_none() {
			error!("Unsupported path: `{}`", self.as_ref().to_string_lossy());
		}
		id
	}

	fn get(self, dictionary: &Dictionary) -> Option<PathKey> {
		let id = self
			.as_ref()
			.as_os_str()
			.to_str()
			.and_then(|s| dictionary.get(s))
			.map(PathKey);
		if id.is_none() {
			error!("Unsupported path: `{}`", self.as_ref().to_string_lossy());
		}
		id
	}
}
