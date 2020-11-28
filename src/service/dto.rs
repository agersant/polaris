use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct Version {
	pub major: i32,
	pub minor: i32,
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct InitialSetup {
	pub has_any_users: bool,
}

#[derive(Serialize, Deserialize)]
pub struct AuthCredentials {
	pub username: String,
	pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct ThumbnailOptions {
	pub pad: Option<bool>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ListPlaylistsEntry {
	pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct SavePlaylistInput {
	pub tracks: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct LastFMLink {
	pub token: String,
	pub content: String,
}
