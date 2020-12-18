use serde::Deserialize;

mod error;
mod manager;

pub use error::*;
pub use manager::*;

#[derive(Clone)]
pub struct AuthSecret {
	pub key: [u8; 32],
}

#[derive(Debug, Queryable)]
struct MiscSettings {
	id: i32,
	auth_secret: Vec<u8>,
	index_sleep_duration_seconds: i32,
	index_album_art_pattern: String,
}

#[derive(Debug)]
pub struct Settings {
	auth_secret: Vec<u8>,
	pub reindex_every_n_seconds: i32,
	pub album_art_pattern: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct NewSettings {
	pub reindex_every_n_seconds: Option<i32>,
	pub album_art_pattern: Option<String>,
}
