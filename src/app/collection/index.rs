use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

use crate::app::collection;

#[derive(Clone, Default)]
pub struct Index {
	lookups: Arc<RwLock<Lookups>>,
}

impl Index {
	pub fn new() -> Self {
		Self::default()
	}

	pub async fn replace_lookup_tables(&mut self, new_lookups: Lookups) {
		let mut lock = self.lookups.write().await;
		*lock = new_lookups;
	}
}

#[derive(Default)]
pub struct Lookups {
	data: HashMap<String, String>,
}

impl Lookups {
	pub fn add_song(&mut self, _song: &collection::Song) {
		// todo!()
	}
}
