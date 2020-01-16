use anyhow::*;
use rustfm_scrobble::{Scrobble, Scrobbler};
use serde::Deserialize;
use std::path::Path;

use crate::db::DB;
use crate::index;
use crate::user;

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

fn scrobble_from_path(db: &DB, track: &Path) -> Result<Scrobble> {
	let song = index::get_song(db, track)?;
	Ok(Scrobble::new(
		song.artist.unwrap_or_else(|| "".into()),
		song.title.unwrap_or_else(|| "".into()),
		song.album.unwrap_or_else(|| "".into()),
	))
}

pub fn link(db: &DB, username: &str, token: &str) -> Result<()> {
	let mut scrobbler = Scrobbler::new(LASTFM_API_KEY.into(), LASTFM_API_SECRET.into());
	let auth_response = scrobbler.authenticate_with_token(token.to_string())?;

	user::lastfm_link(db, username, &auth_response.name, &auth_response.key)
}

pub fn unlink(db: &DB, username: &str) -> Result<()> {
	user::lastfm_unlink(db, username)
}

pub fn scrobble(db: &DB, username: &str, track: &Path) -> Result<()> {
	let mut scrobbler = Scrobbler::new(LASTFM_API_KEY.into(), LASTFM_API_SECRET.into());
	let scrobble = scrobble_from_path(db, track)?;
	let auth_token = user::get_lastfm_session_key(db, username)?;
	scrobbler.authenticate_with_session_key(auth_token);
	scrobbler.scrobble(scrobble)?;
	Ok(())
}

pub fn now_playing(db: &DB, username: &str, track: &Path) -> Result<()> {
	let mut scrobbler = Scrobbler::new(LASTFM_API_KEY.into(), LASTFM_API_SECRET.into());
	let scrobble = scrobble_from_path(db, track)?;
	let auth_token = user::get_lastfm_session_key(db, username)?;
	scrobbler.authenticate_with_session_key(auth_token);
	scrobbler.now_playing(scrobble)?;
	Ok(())
}
