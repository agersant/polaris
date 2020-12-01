use serde::{Deserialize, Serialize};

pub const API_MAJOR_VERSION: i32 = 5;
pub const API_MINOR_VERSION: i32 = 0;
pub const COOKIE_SESSION: &str = "session";
pub const COOKIE_USERNAME: &str = "username";
pub const COOKIE_ADMIN: &str = "admin";

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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ListPlaylistsEntry {
	pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct SavePlaylistInput {
	pub tracks: Vec<String>,
}
