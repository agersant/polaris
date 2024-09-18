use std::{
	collections::{HashMap, HashSet},
	path::{Path, PathBuf},
};

use lasso2::{Rodeo, RodeoReader, Spur};
use log::error;
use serde::{Deserialize, Serialize};
use tinyvec::TinyVec;

use crate::app::scanner;

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum File {
	Directory(PathKey),
	Song(PathKey),
}

#[derive(Serialize, Deserialize)]
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
	pub artists: TinyVec<[Spur; 1]>,
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
	pub artists: TinyVec<[Spur; 1]>,
	pub album_artists: TinyVec<[Spur; 1]>,
	pub year: Option<i64>,
	pub album: Option<Spur>,
	pub artwork: Option<PathKey>,
	pub duration: Option<i64>,
	pub lyricists: TinyVec<[Spur; 0]>,
	pub composers: TinyVec<[Spur; 0]>,
	pub genres: TinyVec<[Spur; 1]>,
	pub labels: TinyVec<[Spur; 0]>,
	pub date_added: i64,
}

#[derive(
	Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
pub struct PathKey(pub Spur);

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct ArtistKey {
	pub name: Option<Spur>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct AlbumKey {
	pub artists: TinyVec<[Spur; 4]>,
	pub name: Spur,
}

#[derive(Copy, Clone, Debug, Default, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct SongKey {
	pub virtual_path: PathKey,
}

impl Song {
	pub fn album_key(&self) -> Option<AlbumKey> {
		let main_artists = match self.album_artists.is_empty() {
			true => &self.artists,
			false => &self.album_artists,
		};

		if main_artists.is_empty() {
			return None;
		}

		match self.album {
			None => None,
			Some(name) => Some(AlbumKey {
				artists: main_artists.iter().cloned().collect(),
				name,
			}),
		}
	}
}

pub fn store_song(
	strings: &mut Rodeo,
	minuscules: &mut HashMap<String, Spur>,
	song: &scanner::Song,
) -> Option<Song> {
	let Some(real_path) = (&song.real_path).get_or_intern(strings) else {
		return None;
	};

	let Some(virtual_path) = (&song.virtual_path).get_or_intern(strings) else {
		return None;
	};

	let artwork = match &song.artwork {
		Some(a) => match a.get_or_intern(strings) {
			Some(a) => Some(a),
			None => return None,
		},
		None => None,
	};

	let mut canonicalize = |s: &String| {
		let mut cleaned = s.clone();
		cleaned.retain(|c| match c {
			' ' | '_' | '-' | '\'' => false,
			_ => true,
		});
		// TODO merge inconsistent diacritic usage
		match cleaned.is_empty() {
			true => None,
			false => Some(
				minuscules
					.entry(cleaned.to_lowercase())
					.or_insert_with(|| strings.get_or_intern(s))
					.to_owned(),
			),
		}
	};

	Some(Song {
		real_path,
		virtual_path,
		track_number: song.track_number,
		disc_number: song.disc_number,
		title: song.title.as_ref().and_then(&mut canonicalize),
		artists: song.artists.iter().filter_map(&mut canonicalize).collect(),
		album_artists: song
			.album_artists
			.iter()
			.filter_map(&mut canonicalize)
			.collect(),
		year: song.year,
		album: song.album.as_ref().and_then(&mut canonicalize),
		artwork: artwork,
		duration: song.duration,
		lyricists: song
			.lyricists
			.iter()
			.filter_map(&mut canonicalize)
			.collect(),
		composers: song
			.composers
			.iter()
			.filter_map(&mut canonicalize)
			.collect(),
		genres: song.genres.iter().filter_map(&mut canonicalize).collect(),
		labels: song.labels.iter().filter_map(&mut canonicalize).collect(),
		date_added: song.date_added,
	})
}

pub fn fetch_song(strings: &RodeoReader, song: &Song) -> super::Song {
	super::Song {
		real_path: PathBuf::from(strings.resolve(&song.real_path.0)),
		virtual_path: PathBuf::from(strings.resolve(&song.virtual_path.0)),
		track_number: song.track_number,
		disc_number: song.disc_number,
		title: song.title.map(|s| strings.resolve(&s).to_string()),
		artists: song
			.artists
			.iter()
			.map(|s| strings.resolve(&s).to_string())
			.collect(),
		album_artists: song
			.album_artists
			.iter()
			.map(|s| strings.resolve(&s).to_string())
			.collect(),
		year: song.year,
		album: song.album.map(|s| strings.resolve(&s).to_string()),
		artwork: song.artwork.map(|a| PathBuf::from(strings.resolve(&a.0))),
		duration: song.duration,
		lyricists: song
			.lyricists
			.iter()
			.map(|s| strings.resolve(&s).to_string())
			.collect(),
		composers: song
			.composers
			.iter()
			.map(|s| strings.resolve(&s).to_string())
			.collect(),
		genres: song
			.genres
			.iter()
			.map(|s| strings.resolve(&s).to_string())
			.collect(),
		labels: song
			.labels
			.iter()
			.map(|s| strings.resolve(&s).to_string())
			.collect(),
		date_added: song.date_added,
	}
}

pub trait InternPath {
	fn get_or_intern(self, strings: &mut Rodeo) -> Option<PathKey>;
	fn get(self, strings: &RodeoReader) -> Option<PathKey>;
}

impl<P: AsRef<Path>> InternPath for P {
	fn get_or_intern(self, strings: &mut Rodeo) -> Option<PathKey> {
		let id = self
			.as_ref()
			.as_os_str()
			.to_str()
			.map(|s| strings.get_or_intern(s))
			.map(PathKey);
		if id.is_none() {
			error!("Unsupported path: `{}`", self.as_ref().to_string_lossy());
		}
		id
	}

	fn get(self, strings: &RodeoReader) -> Option<PathKey> {
		let id = self
			.as_ref()
			.as_os_str()
			.to_str()
			.and_then(|s| strings.get(s))
			.map(PathKey);
		if id.is_none() {
			error!("Unsupported path: `{}`", self.as_ref().to_string_lossy());
		}
		id
	}
}
