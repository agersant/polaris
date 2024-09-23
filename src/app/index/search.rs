use chumsky::Parser;
use lasso2::{RodeoReader, Spur};
use nohash_hasher::{IntMap, IntSet};
use serde::{Deserialize, Serialize};
use std::{
	cmp::Ordering,
	collections::{BTreeMap, HashMap},
};
use tinyvec::TinyVec;

use crate::app::{
	index::{
		query::{BoolOp, Expr, Literal, NumberField, NumberOp, TextField, TextOp},
		storage::SongKey,
	},
	scanner, Error,
};

use super::{
	collection,
	query::make_parser,
	storage::{self, sanitize},
};

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

fn compare_songs(a: &collection::Song, b: &collection::Song) -> Ordering {
	let a_key = {
		let artists = if a.album_artists.is_empty() {
			&a.artists
		} else {
			&a.album_artists
		};
		(artists, a.year, &a.album, a.track_number)
	};

	let b_key = {
		let artists = if b.album_artists.is_empty() {
			&b.artists
		} else {
			&b.album_artists
		};
		(artists, b.year, &b.album, b.track_number)
	};

	a_key.cmp(&b_key)
}

impl Search {
	pub fn find_songs(
		&self,
		collection: &collection::Collection,
		strings: &RodeoReader,
		canon: &HashMap<String, Spur>,
		query: &str,
	) -> Result<Vec<collection::Song>, Error> {
		let parser = make_parser();
		let parsed_query = parser
			.parse(query)
			.map_err(|_| Error::SearchQueryParseError)?;

		let mut songs = self
			.eval(strings, canon, &parsed_query)
			.into_iter()
			.filter_map(|song_key| collection.get_song(strings, song_key))
			.collect::<Vec<_>>();

		songs.sort_by(compare_songs);

		Ok(songs)
	}

	fn eval(
		&self,
		strings: &RodeoReader,
		canon: &HashMap<String, Spur>,
		expr: &Expr,
	) -> IntSet<SongKey> {
		match expr {
			Expr::Fuzzy(s) => self.eval_fuzzy(strings, s),
			Expr::TextCmp(field, op, s) => self.eval_text_operator(strings, canon, *field, *op, &s),
			Expr::NumberCmp(field, op, n) => self.eval_number_operator(*field, *op, *n),
			Expr::Combined(e, op, f) => self.combine(strings, canon, e, *op, f),
		}
	}

	fn combine(
		&self,
		strings: &RodeoReader,
		canon: &HashMap<String, Spur>,
		e: &Box<Expr>,
		op: BoolOp,
		f: &Box<Expr>,
	) -> IntSet<SongKey> {
		match op {
			BoolOp::And => self
				.eval(strings, canon, e)
				.intersection(&self.eval(strings, canon, f))
				.cloned()
				.collect(),
			BoolOp::Or => self
				.eval(strings, canon, e)
				.union(&self.eval(strings, canon, f))
				.cloned()
				.collect(),
		}
	}

	fn eval_fuzzy(&self, strings: &RodeoReader, value: &Literal) -> IntSet<SongKey> {
		match value {
			Literal::Text(s) => {
				let mut songs = IntSet::default();
				for field in self.text_fields.values() {
					songs.extend(field.find_like(strings, s));
				}
				songs
			}
			Literal::Number(n) => {
				let mut songs = IntSet::default();
				for field in self.number_fields.values() {
					songs.extend(field.find(*n as i64, NumberOp::Eq));
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
		canon: &HashMap<String, Spur>,
		field: TextField,
		operator: TextOp,
		value: &str,
	) -> IntSet<SongKey> {
		let Some(field_index) = self.text_fields.get(&field) else {
			return IntSet::default();
		};

		match operator {
			TextOp::Eq => field_index.find_exact(canon, value),
			TextOp::Like => field_index.find_like(strings, value),
		}
	}

	fn eval_number_operator(
		&self,
		field: NumberField,
		operator: NumberOp,
		value: i32,
	) -> IntSet<SongKey> {
		let Some(field_index) = self.number_fields.get(&field) else {
			return IntSet::default();
		};
		field_index.find(value as i64, operator)
	}
}

const NGRAM_SIZE: usize = 2;

#[derive(Default, Deserialize, Serialize)]
struct TextFieldIndex {
	exact: HashMap<Spur, IntSet<SongKey>>,
	ngrams: HashMap<[char; NGRAM_SIZE], IntMap<SongKey, Spur>>,
}

impl TextFieldIndex {
	pub fn insert(&mut self, raw_value: &str, value: Spur, key: SongKey) {
		let characters = sanitize(raw_value).chars().collect::<TinyVec<[char; 32]>>();
		for substring in characters[..].windows(NGRAM_SIZE) {
			self.ngrams
				.entry(substring.try_into().unwrap())
				.or_default()
				.insert(key, value);
		}

		self.exact.entry(value).or_default().insert(key);
	}

	pub fn find_like(&self, strings: &RodeoReader, value: &str) -> IntSet<SongKey> {
		let sanitized = sanitize(value);
		let characters = sanitized.chars().collect::<Vec<_>>();
		let empty = IntMap::default();

		let mut candidates = characters[..]
			.windows(NGRAM_SIZE)
			.map(|s| {
				self.ngrams
					.get::<[char; NGRAM_SIZE]>(s.try_into().unwrap())
					.unwrap_or(&empty)
			})
			.collect::<Vec<_>>();

		if candidates.is_empty() {
			return IntSet::default();
		}

		candidates.sort_by_key(|h| h.len());

		candidates[0]
			.iter()
			// [broad phase] Only keep songs that match all bigrams from the search term
			.filter(move |(song_key, _indexed_value)| {
				candidates[1..].iter().all(|c| c.contains_key(&song_key))
			})
			// [narrow phase] Only keep songs that actually contain the search term in full
			.filter(|(_song_key, indexed_value)| {
				let resolved = strings.resolve(indexed_value);
				sanitize(resolved).contains(&sanitized)
			})
			.map(|(k, _v)| k)
			.copied()
			.collect()
	}

	pub fn find_exact(&self, canon: &HashMap<String, Spur>, value: &str) -> IntSet<SongKey> {
		canon
			.get(&sanitize(value))
			.and_then(|s| self.exact.get(&s))
			.cloned()
			.unwrap_or_default()
	}
}

#[derive(Default, Deserialize, Serialize)]
struct NumberFieldIndex {
	values: BTreeMap<i64, IntSet<SongKey>>,
}

impl NumberFieldIndex {
	pub fn insert(&mut self, value: i64, key: SongKey) {
		self.values.entry(value).or_default().insert(key);
	}

	pub fn find(&self, value: i64, operator: NumberOp) -> IntSet<SongKey> {
		let range = match operator {
			NumberOp::Eq => self.values.range(value..=value),
			NumberOp::Greater => self.values.range((value + 1)..),
			NumberOp::GreaterOrEq => self.values.range(value..),
			NumberOp::Less => self.values.range(..value),
			NumberOp::LessOrEq => self.values.range(..=value),
		};
		let candidates = range.map(|(_n, songs)| songs).collect::<Vec<_>>();
		let mut results = Vec::with_capacity(candidates.iter().map(|c| c.len()).sum());
		candidates
			.into_iter()
			.for_each(|songs| results.extend(songs.iter()));
		IntSet::from_iter(results)
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

		if let Some(disc_number) = &scanner_song.disc_number {
			self.number_fields
				.entry(NumberField::DiscNumber)
				.or_default()
				.insert(*disc_number, song_key);
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

		if let Some(track_number) = &scanner_song.track_number {
			self.number_fields
				.entry(NumberField::TrackNumber)
				.or_default()
				.insert(*track_number, song_key);
		}

		if let Some(year) = &scanner_song.year {
			self.number_fields
				.entry(NumberField::Year)
				.or_default()
				.insert(*year, song_key);
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
	use collection::Collection;

	struct Context {
		canon: HashMap<String, Spur>,
		collection: Collection,
		search: Search,
		strings: RodeoReader,
	}

	impl Context {
		pub fn search(&self, query: &str) -> Vec<PathBuf> {
			self.search
				.find_songs(&self.collection, &self.strings, &self.canon, query)
				.unwrap()
				.into_iter()
				.map(|s| s.virtual_path)
				.collect()
		}
	}

	fn setup_test(songs: Vec<scanner::Song>) -> Context {
		let mut strings = Rodeo::new();
		let mut canon = HashMap::new();

		let mut collection_builder = collection::Builder::default();
		let mut search_builder = Builder::default();
		for song in songs {
			let storage_song = store_song(&mut strings, &mut canon, &song).unwrap();
			collection_builder.add_song(&storage_song);
			search_builder.add_song(&song, &storage_song);
		}

		Context {
			canon,
			collection: collection_builder.build(),
			search: search_builder.build(),
			strings: strings.into_reader(),
		}
	}

	#[test]
	fn can_find_fuzzy() {
		let ctx = setup_test(vec![
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

		let songs = ctx.search("agon");
		assert_eq!(songs.len(), 2);
		assert!(songs.contains(&PathBuf::from("seasons.mp3")));
		assert!(songs.contains(&PathBuf::from("potd.mp3")));
	}

	#[test]
	fn can_find_field_like() {
		let ctx = setup_test(vec![
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

		let songs = ctx.search("artist % agon");
		assert_eq!(songs.len(), 1);
		assert!(songs.contains(&PathBuf::from("seasons.mp3")));
	}

	#[test]
	fn text_is_case_insensitive() {
		let ctx = setup_test(vec![scanner::Song {
			virtual_path: PathBuf::from("seasons.mp3"),
			artists: vec!["Dragonforce".to_owned()],
			..Default::default()
		}]);

		let songs = ctx.search("dragonforce");
		assert_eq!(songs.len(), 1);
		assert!(songs.contains(&PathBuf::from("seasons.mp3")));

		let songs = ctx.search("artist = dragonforce");
		assert_eq!(songs.len(), 1);
		assert!(songs.contains(&PathBuf::from("seasons.mp3")));
	}

	#[test]
	fn can_find_field_exact() {
		let ctx = setup_test(vec![
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

		let songs = ctx.search("artist = Dragon");
		assert!(songs.is_empty());

		let songs = ctx.search("artist = Dragonforce");
		assert_eq!(songs.len(), 1);
		assert!(songs.contains(&PathBuf::from("seasons.mp3")));
	}

	#[test]
	fn can_query_number_fields() {
		let ctx = setup_test(vec![
			scanner::Song {
				virtual_path: PathBuf::from("1999.mp3"),
				year: Some(1999),
				..Default::default()
			},
			scanner::Song {
				virtual_path: PathBuf::from("2000.mp3"),
				year: Some(2000),
				..Default::default()
			},
			scanner::Song {
				virtual_path: PathBuf::from("2001.mp3"),
				year: Some(2001),
				..Default::default()
			},
		]);

		let songs = ctx.search("year=2000");
		assert_eq!(songs.len(), 1);
		assert!(songs.contains(&PathBuf::from("2000.mp3")));

		let songs = ctx.search("year>2000");
		assert_eq!(songs.len(), 1);
		assert!(songs.contains(&PathBuf::from("2001.mp3")));

		let songs = ctx.search("year<2000");
		assert_eq!(songs.len(), 1);
		assert!(songs.contains(&PathBuf::from("1999.mp3")));

		let songs = ctx.search("year>=2000");
		assert_eq!(songs.len(), 2);
		assert!(songs.contains(&PathBuf::from("2000.mp3")));
		assert!(songs.contains(&PathBuf::from("2001.mp3")));

		let songs = ctx.search("year<=2000");
		assert_eq!(songs.len(), 2);
		assert!(songs.contains(&PathBuf::from("1999.mp3")));
		assert!(songs.contains(&PathBuf::from("2000.mp3")));
	}

	#[test]
	fn fuzzy_numbers_query_all_fields() {
		let ctx = setup_test(vec![
			scanner::Song {
				virtual_path: PathBuf::from("music.mp3"),
				year: Some(2000),
				..Default::default()
			},
			scanner::Song {
				virtual_path: PathBuf::from("fireworks 2000.mp3"),
				..Default::default()
			},
			scanner::Song {
				virtual_path: PathBuf::from("calcium.mp3"),
				..Default::default()
			},
		]);

		let songs = ctx.search("2000");
		assert_eq!(songs.len(), 2);
		assert!(songs.contains(&PathBuf::from("music.mp3")));
		assert!(songs.contains(&PathBuf::from("fireworks 2000.mp3")));
	}

	#[test]
	fn can_use_and_operator() {
		let ctx = setup_test(vec![
			scanner::Song {
				virtual_path: PathBuf::from("whale.mp3"),
				..Default::default()
			},
			scanner::Song {
				virtual_path: PathBuf::from("space.mp3"),
				..Default::default()
			},
			scanner::Song {
				virtual_path: PathBuf::from("whales in space.mp3"),
				..Default::default()
			},
		]);

		let songs = ctx.search("space && whale");
		assert_eq!(songs.len(), 1);
		assert!(songs.contains(&PathBuf::from("whales in space.mp3")));

		let songs = ctx.search("space whale");
		assert_eq!(songs.len(), 1);
		assert!(songs.contains(&PathBuf::from("whales in space.mp3")));
	}

	#[test]
	fn can_use_or_operator() {
		let ctx = setup_test(vec![
			scanner::Song {
				virtual_path: PathBuf::from("whale.mp3"),
				..Default::default()
			},
			scanner::Song {
				virtual_path: PathBuf::from("space.mp3"),
				..Default::default()
			},
			scanner::Song {
				virtual_path: PathBuf::from("whales in space.mp3"),
				..Default::default()
			},
		]);

		let songs = ctx.search("space || whale");
		assert_eq!(songs.len(), 3);
		assert!(songs.contains(&PathBuf::from("whale.mp3")));
		assert!(songs.contains(&PathBuf::from("space.mp3")));
		assert!(songs.contains(&PathBuf::from("whales in space.mp3")));
	}

	#[test]
	fn results_are_sorted() {
		let ctx = setup_test(vec![
			scanner::Song {
				virtual_path: PathBuf::from("cry thunder.mp3"),
				artists: vec!["Dragonforce".to_owned()],
				album: Some("The Power Within".to_owned()),
				year: Some(2012),
				genres: vec!["Metal".to_owned()],
				..Default::default()
			},
			scanner::Song {
				virtual_path: PathBuf::from("revelations.mp3"),
				artists: vec!["Dragonforce".to_owned()],
				album: Some("Valley of the Damned".to_owned()),
				year: Some(2003),
				track_number: Some(7),
				genres: vec!["Metal".to_owned()],
				..Default::default()
			},
			scanner::Song {
				virtual_path: PathBuf::from("starfire.mp3"),
				artists: vec!["Dragonforce".to_owned()],
				album: Some("Valley of the Damned".to_owned()),
				year: Some(2003),
				track_number: Some(5),
				genres: vec!["Metal".to_owned()],
				..Default::default()
			},
			scanner::Song {
				virtual_path: PathBuf::from("eternal snow.mp3"),
				artists: vec!["Rhapsody".to_owned()],
				genres: vec!["Metal".to_owned()],
				..Default::default()
			},
			scanner::Song {
				virtual_path: PathBuf::from("alchemy.mp3"),
				artists: vec!["Avantasia".to_owned()],
				genres: vec!["Metal".to_owned()],
				..Default::default()
			},
		]);

		let songs = ctx.search("metal");
		assert_eq!(songs.len(), 5);
		assert_eq!(
			songs,
			vec![
				PathBuf::from("alchemy.mp3"),
				PathBuf::from("starfire.mp3"),
				PathBuf::from("revelations.mp3"),
				PathBuf::from("cry thunder.mp3"),
				PathBuf::from("eternal snow.mp3"),
			]
		);
	}

	#[test]
	fn avoids_bigram_false_positives() {
		let ctx = setup_test(vec![scanner::Song {
			virtual_path: PathBuf::from("lorry bovine vehicle.mp3"),
			..Default::default()
		}]);

		let songs = ctx.search("love");
		assert!(songs.is_empty());
	}
}
