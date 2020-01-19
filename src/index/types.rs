use serde::{Deserialize, Serialize};

use crate::db::songs;

#[derive(Debug, PartialEq, Queryable, QueryableByName, Serialize, Deserialize)]
#[table_name = "songs"]
pub struct Song {
	#[serde(skip_serializing, skip_deserializing)]
	id: i32,
	pub path: String,
	#[serde(skip_serializing, skip_deserializing)]
	pub parent: String,
	pub track_number: Option<i32>,
	pub disc_number: Option<i32>,
	pub title: Option<String>,
	pub artist: Option<String>,
	pub album_artist: Option<String>,
	pub year: Option<i32>,
	pub album: Option<String>,
	pub artwork: Option<String>,
	pub duration: Option<i32>,
}

#[derive(Debug, PartialEq, Queryable, Serialize, Deserialize)]
pub struct Directory {
	#[serde(skip_serializing, skip_deserializing)]
	id: i32,
	pub path: String,
	#[serde(skip_serializing, skip_deserializing)]
	pub parent: Option<String>,
	pub artist: Option<String>,
	pub year: Option<i32>,
	pub album: Option<String>,
	pub artwork: Option<String>,
	pub date_added: i32,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum CollectionFile {
	Directory(Directory),
	Song(Song),
}