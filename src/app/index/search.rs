use chumsky::Parser;
use lasso2::{RodeoReader, Spur};
use serde::{Deserialize, Serialize};
use std::{
	collections::{HashMap, HashSet},
	ffi::OsStr,
	path::{Path, PathBuf},
};
use tinyvec::TinyVec;

use crate::app::{
	index::{
		query::{BoolOp, Expr, Literal, NumberField, NumberOp, TextField, TextOp},
		storage::SongKey,
	},
	scanner, Error,
};

use super::{query::make_parser, storage};

#[derive(Serialize, Deserialize)]
pub struct Search {
	text_fields: HashMap<TextField, TextFieldIndex>,
	number_fields: HashMap<NumberField, NumberFieldIndex>,
}

impl Default for Search {
	fn default() -> Self {
		Self {
			text_fields: Default::default(),
			number_fields: Default::default(),
		}
	}
}

impl Search {
	pub fn find_songs(&self, strings: &RodeoReader, query: &str) -> Result<Vec<PathBuf>, Error> {
		let parser = make_parser();
		let parsed_query = parser
			.parse(query)
			.map_err(|_| Error::SearchQueryParseError)?;

		let keys = self.eval(strings, &parsed_query);
		Ok(keys
			.into_iter()
			.map(|k| Path::new(OsStr::new(strings.resolve(&k.virtual_path.0))).to_owned())
			.collect::<Vec<_>>())
	}

	fn eval(&self, strings: &RodeoReader, expr: &Expr) -> HashSet<SongKey> {
		match expr {
			Expr::Fuzzy(s) => self.eval_fuzzy(strings, s),
			Expr::TextCmp(field, op, s) => self.eval_text_operator(strings, *field, *op, &s),
			Expr::NumberCmp(field, op, n) => self.eval_number_operator(*field, *op, *n),
			Expr::Combined(e, op, f) => self.combine(strings, e, *op, f),
		}
	}

	fn combine(
		&self,
		strings: &RodeoReader,
		e: &Box<Expr>,
		op: BoolOp,
		f: &Box<Expr>,
	) -> HashSet<SongKey> {
		match op {
			BoolOp::And => self
				.eval(strings, e)
				.intersection(&self.eval(strings, f))
				.cloned()
				.collect(),
			BoolOp::Or => self
				.eval(strings, e)
				.union(&self.eval(strings, f))
				.cloned()
				.collect(),
		}
	}

	fn eval_fuzzy(&self, strings: &RodeoReader, value: &Literal) -> HashSet<SongKey> {
		match value {
			Literal::Text(s) => {
				let mut songs = HashSet::new();
				for field in self.text_fields.values() {
					songs.extend(field.find_like(strings, s));
				}
				songs
			}
			Literal::Number(n) => {
				let mut songs = HashSet::new();
				for field in self.number_fields.values() {
					songs.extend(field.find_equal(*n));
				}
				songs
					.union(&self.eval_fuzzy(strings, &Literal::Text(n.to_string())))
					.copied()
					.collect()
			}
		}
	}

	fn eval_text_operator(
		&self,
		strings: &RodeoReader,
		field: TextField,
		operator: TextOp,
		value: &str,
	) -> HashSet<SongKey> {
		let Some(field_index) = self.text_fields.get(&field) else {
			return HashSet::new();
		};

		match operator {
			TextOp::Eq => field_index.find_exact(strings, value),
			TextOp::Like => field_index.find_like(strings, value),
		}
	}

	fn eval_number_operator(
		&self,
		field: NumberField,
		operator: NumberOp,
		value: i32,
	) -> HashSet<SongKey> {
		todo!()
	}
}

const NGRAM_SIZE: usize = 2;

#[derive(Default, Deserialize, Serialize)]
struct TextFieldIndex {
	exact: HashMap<Spur, HashSet<SongKey>>,
	ngrams: HashMap<[char; NGRAM_SIZE], HashSet<SongKey>>,
}

impl TextFieldIndex {
	pub fn insert(&mut self, raw_value: &str, value: Spur, key: SongKey) {
		// TODO sanitize ngrams
		let characters = raw_value.chars().collect::<TinyVec<[char; 32]>>();
		for substring in characters[..].windows(NGRAM_SIZE) {
			self.ngrams
				.entry(substring.try_into().unwrap())
				.or_default()
				.insert(key);
		}

		self.exact.entry(value).or_default().insert(key);
	}

	pub fn find_like(&self, strings: &RodeoReader, value: &str) -> HashSet<SongKey> {
		let characters = value.chars().collect::<Vec<_>>();
		let empty_set = HashSet::new();

		let mut candidates = characters[..]
			.windows(NGRAM_SIZE)
			.map(|s| {
				self.ngrams
					.get::<[char; NGRAM_SIZE]>(s.try_into().unwrap())
					.unwrap_or(&empty_set)
			})
			.collect::<Vec<_>>();

		if candidates.is_empty() {
			return HashSet::new();
		}

		candidates.sort_by_key(|h| h.len());

		candidates[0]
			.iter()
			.filter(move |c| candidates[1..].iter().all(|s| s.contains(c)))
			.filter(|s| strings.resolve(&s.virtual_path.0).contains(value))
			.copied()
			.collect()
	}

	pub fn find_exact(&self, strings: &RodeoReader, value: &str) -> HashSet<SongKey> {
		strings
			.get(value)
			.and_then(|k| self.exact.get(&k))
			.cloned()
			.unwrap_or_default()
	}
}

#[derive(Default, Deserialize, Serialize)]
struct NumberFieldIndex {
	values: HashMap<i32, HashSet<SongKey>>,
}

impl NumberFieldIndex {
	pub fn insert(&mut self, raw_value: &str, value: Spur, key: SongKey) {}

	pub fn find_equal(&self, value: i32) -> HashSet<SongKey> {
		todo!()
	}
}

#[derive(Default)]
pub struct Builder {
	text_fields: HashMap<TextField, TextFieldIndex>,
	number_fields: HashMap<NumberField, NumberFieldIndex>,
}

impl Builder {
	pub fn add_song(&mut self, scanner_song: &scanner::Song, storage_song: &storage::Song) {
		let song_key = SongKey {
			virtual_path: storage_song.virtual_path,
		};

		if let (Some(str), Some(spur)) = (&scanner_song.album, storage_song.album) {
			self.text_fields
				.entry(TextField::Album)
				.or_default()
				.insert(str, spur, song_key);
		}

		for (str, spur) in scanner_song
			.album_artists
			.iter()
			.zip(storage_song.album_artists.iter())
		{
			self.text_fields
				.entry(TextField::AlbumArtist)
				.or_default()
				.insert(str, *spur, song_key);
		}

		for (str, spur) in scanner_song.artists.iter().zip(storage_song.artists.iter()) {
			self.text_fields
				.entry(TextField::Artist)
				.or_default()
				.insert(str, *spur, song_key);
		}

		for (str, spur) in scanner_song
			.composers
			.iter()
			.zip(storage_song.composers.iter())
		{
			self.text_fields
				.entry(TextField::Composer)
				.or_default()
				.insert(str, *spur, song_key);
		}

		for (str, spur) in scanner_song.genres.iter().zip(storage_song.genres.iter()) {
			self.text_fields
				.entry(TextField::Genre)
				.or_default()
				.insert(str, *spur, song_key);
		}

		for (str, spur) in scanner_song.labels.iter().zip(storage_song.labels.iter()) {
			self.text_fields
				.entry(TextField::Label)
				.or_default()
				.insert(str, *spur, song_key);
		}

		for (str, spur) in scanner_song
			.lyricists
			.iter()
			.zip(storage_song.lyricists.iter())
		{
			self.text_fields
				.entry(TextField::Lyricist)
				.or_default()
				.insert(str, *spur, song_key);
		}

		self.text_fields.entry(TextField::Path).or_default().insert(
			scanner_song.virtual_path.to_string_lossy().as_ref(),
			storage_song.virtual_path.0,
			song_key,
		);

		if let (Some(str), Some(spur)) = (&scanner_song.title, storage_song.title) {
			self.text_fields
				.entry(TextField::Title)
				.or_default()
				.insert(str, spur, song_key);
		}
	}

	pub fn build(self) -> Search {
		Search {
			text_fields: self.text_fields,
			number_fields: self.number_fields,
		}
	}
}

#[cfg(test)]
mod test {
	use std::path::PathBuf;

	use lasso2::Rodeo;
	use storage::store_song;

	use super::*;

	fn setup_test(songs: Vec<scanner::Song>) -> (Search, RodeoReader) {
		let mut strings = Rodeo::new();
		let mut canon = HashMap::new();

		let mut builder = Builder::default();
		for song in songs {
			let storage_song = store_song(&mut strings, &mut canon, &song).unwrap();
			builder.add_song(&song, &storage_song);
		}

		let search = builder.build();
		let strings = strings.into_reader();
		(search, strings)
	}

	#[test]
	fn can_find_fuzzy() {
		let (search, strings) = setup_test(vec![
			scanner::Song {
				virtual_path: PathBuf::from("seasons.mp3"),
				title: Some("Seasons".to_owned()),
				artists: vec!["Dragonforce".to_owned()],
				..Default::default()
			},
			scanner::Song {
				virtual_path: PathBuf::from("potd.mp3"),
				title: Some("Power of the Dragonflame".to_owned()),
				artists: vec!["Rhapsody".to_owned()],
				..Default::default()
			},
			scanner::Song {
				virtual_path: PathBuf::from("calcium.mp3"),
				title: Some("Calcium".to_owned()),
				artists: vec!["FSOL".to_owned()],
				..Default::default()
			},
		]);

		let songs = search.find_songs(&strings, "agon").unwrap();

		assert_eq!(songs.len(), 2);
		assert!(songs.contains(&PathBuf::from("seasons.mp3")));
		assert!(songs.contains(&PathBuf::from("potd.mp3")));
	}

	#[test]
	fn can_find_field_like() {
		let (search, strings) = setup_test(vec![
			scanner::Song {
				virtual_path: PathBuf::from("seasons.mp3"),
				title: Some("Seasons".to_owned()),
				artists: vec!["Dragonforce".to_owned()],
				..Default::default()
			},
			scanner::Song {
				virtual_path: PathBuf::from("potd.mp3"),
				title: Some("Power of the Dragonflame".to_owned()),
				artists: vec!["Rhapsody".to_owned()],
				..Default::default()
			},
		]);

		let songs = search.find_songs(&strings, "artist % agon").unwrap();

		assert_eq!(songs.len(), 1);
		assert!(songs.contains(&PathBuf::from("seasons.mp3")));
	}
}
