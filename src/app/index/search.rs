use chumsky::Parser;
use enum_map::EnumMap;
use lasso2::Spur;
use nohash_hasher::IntSet;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use tinyvec::TinyVec;

use crate::app::{
	index::{
		dictionary::Dictionary,
		query::{BoolOp, Expr, Literal, NumberField, NumberOp, TextField, TextOp},
		storage::SongKey,
	},
	scanner, Error,
};

use super::{collection, dictionary::sanitize, query::make_parser, storage};

#[derive(Serialize, Deserialize)]
pub struct Search {
	text_fields: EnumMap<TextField, TextFieldIndex>,
	number_fields: EnumMap<NumberField, NumberFieldIndex>,
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
	pub fn find_songs(
		&self,
		collection: &collection::Collection,
		dictionary: &Dictionary,
		query: &str,
	) -> Result<Vec<collection::Song>, Error> {
		let parser = make_parser();
		let parsed_query = parser
			.parse(query)
			.map_err(|_| Error::SearchQueryParseError)?;

		let mut songs = self
			.eval(dictionary, &parsed_query)
			.into_iter()
			.collect::<Vec<_>>();
		collection.sort_songs(&mut songs, dictionary);
		let songs = songs
			.into_iter()
			.filter_map(|song_key| collection.get_song(dictionary, song_key))
			.collect::<Vec<_>>();

		Ok(songs)
	}

	fn eval(&self, dictionary: &Dictionary, expr: &Expr) -> IntSet<SongKey> {
		match expr {
			Expr::Fuzzy(s) => self.eval_fuzzy(dictionary, s),
			Expr::TextCmp(field, op, s) => self.eval_text_operator(dictionary, *field, *op, s),
			Expr::NumberCmp(field, op, n) => self.eval_number_operator(*field, *op, *n),
			Expr::Combined(e, op, f) => self.combine(dictionary, e, *op, f),
		}
	}

	fn combine(
		&self,
		dictionary: &Dictionary,
		e: &Expr,
		op: BoolOp,
		f: &Expr,
	) -> IntSet<SongKey> {
		let is_operable = |expr: &Expr| match expr {
			Expr::Fuzzy(Literal::Text(s)) if s.chars().count() < BIGRAM_SIZE => false,
			Expr::Fuzzy(Literal::Number(n)) if *n < 10 => false,
			Expr::TextCmp(_, _, s) if s.chars().count() < BIGRAM_SIZE => false,
			_ => true,
		};

		let left = is_operable(e).then(|| self.eval(dictionary, e));
		let right = is_operable(f).then(|| self.eval(dictionary, f));

		match (left, op, right) {
			(Some(l), BoolOp::And, Some(r)) => l.intersection(&r).cloned().collect(),
			(Some(l), BoolOp::Or, Some(r)) => l.union(&r).cloned().collect(),
			(Some(l), BoolOp::Not, Some(r)) => l.difference(&r).cloned().collect(),
			(None, BoolOp::Not, _) => IntSet::default(),
			(Some(l), _, None) => l,
			(None, _, Some(r)) => r,
			(None, _, None) => IntSet::default(),
		}
	}

	fn eval_fuzzy(&self, dictionary: &Dictionary, value: &Literal) -> IntSet<SongKey> {
		match value {
			Literal::Text(s) => {
				let mut songs = IntSet::default();
				for field in self.text_fields.values() {
					songs.extend(field.find_like(dictionary, s));
				}
				songs
			}
			Literal::Number(n) => {
				let mut songs = IntSet::default();
				for field in self.number_fields.values() {
					songs.extend(field.find(*n as i64, NumberOp::Eq));
				}
				songs
					.union(&self.eval_fuzzy(dictionary, &Literal::Text(n.to_string())))
					.copied()
					.collect()
			}
		}
	}

	fn eval_text_operator(
		&self,
		dictionary: &Dictionary,
		field: TextField,
		operator: TextOp,
		value: &str,
	) -> IntSet<SongKey> {
		match operator {
			TextOp::Eq => self.text_fields[field].find_exact(dictionary, value),
			TextOp::Like => self.text_fields[field].find_like(dictionary, value),
		}
	}

	fn eval_number_operator(
		&self,
		field: NumberField,
		operator: NumberOp,
		value: i32,
	) -> IntSet<SongKey> {
		self.number_fields[field].find(value as i64, operator)
	}
}

const BIGRAM_SIZE: usize = 2;
const ASCII_RANGE: usize = u8::MAX as usize;

#[derive(Clone, Deserialize, Serialize)]
struct TextFieldIndex {
	exact: HashMap<Spur, IntSet<SongKey>>,
	ascii_bigrams: Vec<Vec<(SongKey, Spur)>>,
	other_bigrams: HashMap<[char; BIGRAM_SIZE], Vec<(SongKey, Spur)>>,
}

impl Default for TextFieldIndex {
	fn default() -> Self {
		Self {
			exact: Default::default(),
			ascii_bigrams: vec![Default::default(); ASCII_RANGE * ASCII_RANGE],
			other_bigrams: Default::default(),
		}
	}
}

impl TextFieldIndex {
	fn ascii_bigram_to_index(a: char, b: char) -> usize {
		assert!(a.is_ascii());
		assert!(b.is_ascii());
		(a as usize) * ASCII_RANGE + (b as usize)
	}

	pub fn insert(&mut self, raw_value: &str, value: Spur, song: SongKey) {
		let characters = sanitize(raw_value).chars().collect::<TinyVec<[char; 32]>>();
		for substring in characters[..].windows(BIGRAM_SIZE) {
			if substring.iter().all(|c| c.is_ascii()) {
				let index = Self::ascii_bigram_to_index(substring[0], substring[1]);
				self.ascii_bigrams[index].push((song, value));
			} else {
				self.other_bigrams
					.entry(substring.try_into().unwrap())
					.or_default()
					.push((song, value));
			}
		}

		self.exact.entry(value).or_default().insert(song);
	}

	pub fn find_like(&self, dictionary: &Dictionary, value: &str) -> IntSet<SongKey> {
		let sanitized = sanitize(value);
		let characters = sanitized.chars().collect::<Vec<_>>();
		let empty = Vec::new();

		let candidates_by_bigram = characters[..]
			.windows(BIGRAM_SIZE)
			.map(|s| {
				if s.iter().all(|c| c.is_ascii()) {
					let index = Self::ascii_bigram_to_index(s[0], s[1]);
					&self.ascii_bigrams[index]
				} else {
					self.other_bigrams
						.get::<[char; BIGRAM_SIZE]>(s.try_into().unwrap())
						.unwrap_or(&empty)
				}
			})
			.collect::<Vec<_>>();

		candidates_by_bigram
			.into_iter()
			.min_by_key(|h| h.len()) // Only check songs that contain the least common bigram from the search term
			.unwrap_or(&empty)
			.iter()
			.filter(|(_song_key, indexed_value)| {
				// Only keep songs that actually contain the search term in full
				let resolved = dictionary.resolve(indexed_value);
				sanitize(resolved).contains(&sanitized)
			})
			.map(|(k, _v)| k)
			.copied()
			.collect()
	}

	pub fn find_exact(&self, dictionary: &Dictionary, value: &str) -> IntSet<SongKey> {
		dictionary
			.get_canon(value)
			.and_then(|s| self.exact.get(&s))
			.cloned()
			.unwrap_or_default()
	}
}

#[derive(Clone, Default, Deserialize, Serialize)]
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

#[derive(Clone, Default)]
pub struct Builder {
	text_fields: EnumMap<TextField, TextFieldIndex>,
	number_fields: EnumMap<NumberField, NumberFieldIndex>,
}

impl Builder {
	pub fn add_song(&mut self, scanner_song: &scanner::Song, storage_song: &storage::Song) {
		let song_key = SongKey {
			virtual_path: storage_song.virtual_path,
		};

		if let (Some(str), Some(spur)) = (&scanner_song.album, storage_song.album) {
			self.text_fields[TextField::Album].insert(str, spur, song_key);
		}

		for (str, artist_key) in scanner_song
			.album_artists
			.iter()
			.zip(storage_song.album_artists.iter())
		{
			self.text_fields[TextField::AlbumArtist].insert(str, artist_key.0, song_key);
		}

		for (str, artist_key) in scanner_song.artists.iter().zip(storage_song.artists.iter()) {
			self.text_fields[TextField::Artist].insert(str, artist_key.0, song_key);
		}

		for (str, artist_key) in scanner_song
			.composers
			.iter()
			.zip(storage_song.composers.iter())
		{
			self.text_fields[TextField::Composer].insert(str, artist_key.0, song_key);
		}

		if let Some(disc_number) = &scanner_song.disc_number {
			self.number_fields[NumberField::DiscNumber].insert(*disc_number, song_key);
		}

		for (str, spur) in scanner_song.genres.iter().zip(storage_song.genres.iter()) {
			self.text_fields[TextField::Genre].insert(str, *spur, song_key);
		}

		for (str, spur) in scanner_song.labels.iter().zip(storage_song.labels.iter()) {
			self.text_fields[TextField::Label].insert(str, *spur, song_key);
		}

		for (str, artist_key) in scanner_song
			.lyricists
			.iter()
			.zip(storage_song.lyricists.iter())
		{
			self.text_fields[TextField::Lyricist].insert(str, artist_key.0, song_key);
		}

		self.text_fields[TextField::Path].insert(
			scanner_song.virtual_path.to_string_lossy().as_ref(),
			storage_song.virtual_path.0,
			song_key,
		);

		if let (Some(str), Some(spur)) = (&scanner_song.title, storage_song.title) {
			self.text_fields[TextField::Title].insert(str, spur, song_key);
		}

		if let Some(track_number) = &scanner_song.track_number {
			self.number_fields[NumberField::TrackNumber].insert(*track_number, song_key);
		}

		if let Some(year) = &scanner_song.year {
			self.number_fields[NumberField::Year].insert(*year, song_key);
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

	use super::*;
	use crate::app::index::dictionary;
	use collection::Collection;
	use storage::store_song;

	struct Context {
		dictionary: Dictionary,
		collection: Collection,
		search: Search,
	}

	impl Context {
		pub fn search(&self, query: &str) -> Vec<PathBuf> {
			self.search
				.find_songs(&self.collection, &self.dictionary, query)
				.unwrap()
				.into_iter()
				.map(|s| s.virtual_path)
				.collect()
		}
	}

	fn setup_test(songs: Vec<scanner::Song>) -> Context {
		let mut dictionary_builder = dictionary::Builder::default();
		let mut collection_builder = collection::Builder::default();
		let mut search_builder = Builder::default();
		for song in songs {
			let storage_song = store_song(&mut dictionary_builder, &song).unwrap();
			collection_builder.add_song(&storage_song);
			search_builder.add_song(&song, &storage_song);
		}

		Context {
			collection: collection_builder.build(),
			search: search_builder.build(),
			dictionary: dictionary_builder.build(),
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
	fn can_use_not_operator() {
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

		let songs = ctx.search("whale !! space");
		assert_eq!(songs.len(), 1);
		assert!(songs.contains(&PathBuf::from("whale.mp3")));
	}

	#[test]
	fn results_are_sorted() {
		let ctx = setup_test(vec![
			scanner::Song {
				virtual_path: PathBuf::from("accented.mp3"),
				artists: vec!["à la maison".to_owned()],
				genres: vec!["Metal".to_owned()],
				..Default::default()
			},
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
		assert_eq!(songs.len(), 6);
		assert_eq!(
			songs,
			vec![
				PathBuf::from("accented.mp3"),
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

	#[test]
	fn ignores_single_letter_components() {
		let ctx = setup_test(vec![scanner::Song {
			virtual_path: PathBuf::from("seasons.mp3"),
			..Default::default()
		}]);

		let songs = ctx.search("seas u");
		assert_eq!(songs.len(), 1);

		let songs = ctx.search("seas 2");
		assert_eq!(songs.len(), 1);

		let songs = ctx.search("seas || u");
		assert_eq!(songs.len(), 1);

		let songs = ctx.search("seas || 2");
		assert_eq!(songs.len(), 1);
	}
}
