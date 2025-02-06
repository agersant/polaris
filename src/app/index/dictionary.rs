use std::{cmp::Ordering, collections::HashMap};

use icu_collator::{Collator, CollatorOptions, Strength};
use lasso2::{Rodeo, RodeoReader, Spur};
use rayon::slice::ParallelSliceMut;
use serde::{Deserialize, Serialize};

pub fn sanitize(s: &str) -> String {
	// TODO merge inconsistent diacritic usage
	let mut cleaned = s.to_owned();
	cleaned.retain(|c| !matches!(c, ' ' | '_' | '-' | '\''));
	cleaned.to_lowercase()
}

pub fn make_collator() -> Collator {
	let options = {
		let mut o = CollatorOptions::new();
		o.strength = Some(Strength::Secondary);
		o
	};
	Collator::try_new(&Default::default(), options).unwrap()
}

#[derive(Serialize, Deserialize)]
pub struct Dictionary {
	strings: RodeoReader,          // Interned strings
	canon: HashMap<String, Spur>,  // Canonical representation of similar strings
	sort_keys: HashMap<Spur, u32>, // All spurs sorted against each other
}

impl Dictionary {
	pub fn get<S: AsRef<str>>(&self, string: S) -> Option<Spur> {
		self.strings.get(string)
	}

	pub fn get_canon<S: AsRef<str>>(&self, string: S) -> Option<Spur> {
		self.canon.get(&sanitize(string.as_ref())).copied()
	}

	pub fn resolve(&self, spur: &Spur) -> &str {
		self.strings.resolve(spur)
	}

	pub fn cmp(&self, a: &Spur, b: &Spur) -> Ordering {
		self.sort_keys
			.get(a)
			.copied()
			.unwrap_or_default()
			.cmp(&self.sort_keys.get(b).copied().unwrap_or_default())
	}
}

impl Default for Dictionary {
	fn default() -> Self {
		Self {
			strings: Rodeo::default().into_reader(),
			canon: Default::default(),
			sort_keys: Default::default(),
		}
	}
}

#[derive(Clone, Default)]
pub struct Builder {
	strings: Rodeo,
	canon: HashMap<String, Spur>,
}

impl Builder {
	pub fn build(self) -> Dictionary {
		let mut sorted_spurs = self.strings.iter().collect::<Vec<_>>();
		// TODO this is too slow!
		sorted_spurs.par_sort_unstable_by(|(_, a), (_, b)| {
			let collator = make_collator();
			collator.compare(a, b)
		});

		let sort_keys = sorted_spurs
			.into_iter()
			.enumerate()
			.map(|(i, (spur, _))| (spur, i as u32))
			.collect();

		Dictionary {
			strings: self.strings.into_reader(),
			canon: self.canon,
			sort_keys,
		}
	}

	pub fn get_or_intern<S: AsRef<str>>(&mut self, string: S) -> Spur {
		self.strings.get_or_intern(string)
	}

	pub fn get_or_intern_canon<S: AsRef<str>>(&mut self, string: S) -> Option<Spur> {
		let cleaned = sanitize(string.as_ref());
		match cleaned.is_empty() {
			true => None,
			false => Some(
				self.canon
					.entry(cleaned)
					.or_insert_with(|| self.strings.get_or_intern(string.as_ref()))
					.to_owned(),
			),
		}
	}
}
