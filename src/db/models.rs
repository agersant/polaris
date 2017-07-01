// Collection content
#[derive(Debug, Queryable, Serialize)]
pub struct Song {
	#[serde(skip_serializing)]
	id: i32,
	pub path: String,
	#[serde(skip_serializing)]
	pub parent: String,
	pub track_number: Option<i32>,
	pub disc_number: Option<i32>,
	pub title: Option<String>,
	pub artist: Option<String>,
	pub album_artist: Option<String>,
	pub year: Option<i32>,
	pub album: Option<String>,
	pub artwork: Option<String>,
}

#[derive(Debug, Queryable, Serialize)]
pub struct Directory {
	#[serde(skip_serializing)]
	id: i32,
	pub path: String,
	#[serde(skip_serializing)]
	pub parent: Option<String>,
	pub artist: Option<String>,
	pub year: Option<i32>,
	pub album: Option<String>,
	pub artwork: Option<String>,
	pub date_added: i32,
}

#[derive(Debug, Serialize)]
pub enum CollectionFile {
	Directory(Directory),
	Song(Song),
}

// Misc Settings
#[derive(Debug, Queryable)]
pub struct MiscSettings {
	id: i32,
	pub auth_secret: String,
	pub index_sleep_duration_seconds: i32,
	pub index_album_art_pattern: String,
}
