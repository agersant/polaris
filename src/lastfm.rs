use rustfm_scrobble::{Scrobble, Scrobbler};
use std::path::Path;

use crate::db::ConnectionSource;
use crate::errors;
use crate::index;
use crate::user;
use crate::vfs::VFSSource;

const LASTFM_API_KEY: &str = "02b96c939a2b451c31dfd67add1f696e";
const LASTFM_API_SECRET: &str = "0f25a80ceef4b470b5cb97d99d4b3420";

#[derive(Debug, Deserialize)]
struct AuthResponseSessionName {
	#[serde(rename = "$value")]
	pub body: String,
}

#[derive(Debug, Deserialize)]
struct AuthResponseSessionKey {
	#[serde(rename = "$value")]
	pub body: String,
}

#[derive(Debug, Deserialize)]
struct AuthResponseSessionSubscriber {
	#[serde(rename = "$value")]
	pub body: i32,
}

#[derive(Debug, Deserialize)]
struct AuthResponseSession {
	pub name: AuthResponseSessionName,
	pub key: AuthResponseSessionKey,
	pub subscriber: AuthResponseSessionSubscriber,
}

#[derive(Debug, Deserialize)]
struct AuthResponse {
	pub status: String,
	pub session: AuthResponseSession,
}

fn scrobble_from_path<T>(db: &T, track: &Path) -> Result<Scrobble, errors::Error>
where
	T: ConnectionSource + VFSSource,
{
	let song = index::get_song(db, track)?;
	Ok(Scrobble::new(
		song.artist.unwrap_or_else(|| "".into()),
		song.title.unwrap_or_else(|| "".into()),
		song.album.unwrap_or_else(|| "".into()),
	))
}

pub fn link<T>(db: &T, username: &str, token: &str) -> Result<(), errors::Error>
where
	T: ConnectionSource + VFSSource,
{
	let mut scrobbler = Scrobbler::new(LASTFM_API_KEY.into(), LASTFM_API_SECRET.into());
	let auth_response = scrobbler.authenticate_with_token(token.to_string())?;

	user::lastfm_link(db, username, &auth_response.name, &auth_response.key)
}

pub fn unlink<T>(db: &T, username: &str) -> Result<(), errors::Error>
where
	T: ConnectionSource + VFSSource,
{
	user::lastfm_unlink(db, username)
}

pub fn scrobble<T>(db: &T, username: &str, track: &Path) -> Result<(), errors::Error>
where
	T: ConnectionSource + VFSSource,
{
	let mut scrobbler = Scrobbler::new(LASTFM_API_KEY.into(), LASTFM_API_SECRET.into());
	let scrobble = scrobble_from_path(db, track)?;
	let auth_token = user::get_lastfm_session_key(db, username)?;
	scrobbler.authenticate_with_session_key(auth_token);
	scrobbler.scrobble(scrobble)?;
	Ok(())
}

pub fn now_playing<T>(db: &T, username: &str, track: &Path) -> Result<(), errors::Error>
where
	T: ConnectionSource + VFSSource,
{
	let mut scrobbler = Scrobbler::new(LASTFM_API_KEY.into(), LASTFM_API_SECRET.into());
	let scrobble = scrobble_from_path(db, track)?;
	let auth_token = user::get_lastfm_session_key(db, username)?;
	scrobbler.authenticate_with_session_key(auth_token);
	scrobbler.now_playing(scrobble)?;
	Ok(())
}
