use md5;
use reqwest;
use rustfm_scrobble::{Scrobble, Scrobbler};
use serde_xml_rs::deserialize;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use crate::db::ConnectionSource;
use crate::errors;
use crate::index;
use crate::user;
use crate::vfs::VFSSource;

const LASTFM_API_KEY: &str = "02b96c939a2b451c31dfd67add1f696e";
const LASTFM_API_SECRET: &str = "0f25a80ceef4b470b5cb97d99d4b3420";
const LASTFM_API_ROOT: &str = "http://ws.audioscrobbler.com/2.0/";

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
	let mut params = HashMap::new();
	params.insert("token".to_string(), token.to_string());
	params.insert("api_key".to_string(), LASTFM_API_KEY.to_string());
	let mut response = match api_request("auth.getSession", &params) {
		Ok(r) => r,
		Err(_) => bail!(errors::ErrorKind::LastFMAuthError),
	};

	let mut body = String::new();
	response.read_to_string(&mut body)?;
	if !response.status().is_success() {
		bail!(errors::ErrorKind::LastFMAuthError)
	}

	let auth_response: AuthResponse = match deserialize(body.as_bytes()) {
		Ok(d) => d,
		Err(_) => bail!(errors::ErrorKind::LastFMDeserializationError),
	};

	user::lastfm_link(
		db,
		username,
		&auth_response.session.name.body,
		&auth_response.session.key.body,
	)
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

fn api_request(
	method: &str,
	params: &HashMap<String, String>,
) -> Result<reqwest::Response, reqwest::Error> {
	let mut url = LASTFM_API_ROOT.to_string();
	url.push_str("?");

	url.push_str(&format!("method={}&", method));
	for (k, v) in params.iter() {
		url.push_str(&format!("{}={}&", k, v));
	}
	let api_signature = get_signature(method, params);
	url.push_str(&format!("api_sig={}", api_signature));

	let client = reqwest::ClientBuilder::new().build()?;
	let request = client.get(url.as_str());
	request.send()
}

fn get_signature(method: &str, params: &HashMap<String, String>) -> String {
	let mut signature_data = params.clone();
	signature_data.insert("method".to_string(), method.to_string());

	let mut param_names = Vec::new();
	for param_name in signature_data.keys() {
		param_names.push(param_name);
	}
	param_names.sort();

	let mut signature = String::new();
	for param_name in param_names {
		signature.push_str((param_name.to_string() + signature_data[param_name].as_str()).as_str())
	}

	signature.push_str(LASTFM_API_SECRET);

	let digest = md5::compute(signature.as_bytes());
	format!("{:X}", digest)
}
