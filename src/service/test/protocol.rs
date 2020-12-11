use http::{method::Method, Request};
use percent_encoding::{percent_encode, NON_ALPHANUMERIC};
use std::path::Path;

use crate::app::{config, user};
use crate::service::dto;

pub fn web_index() -> Request<()> {
	Request::builder()
		.method(Method::GET)
		.uri("/")
		.body(())
		.unwrap()
}

pub fn swagger_index() -> Request<()> {
	Request::builder()
		.method(Method::GET)
		.uri("/swagger")
		.body(())
		.unwrap()
}

pub fn version() -> Request<()> {
	Request::builder()
		.method(Method::GET)
		.uri("/api/version")
		.body(())
		.unwrap()
}

pub fn initial_setup() -> Request<()> {
	Request::builder()
		.method(Method::GET)
		.uri("/api/initial_setup")
		.body(())
		.unwrap()
}

pub fn login(username: &str, password: &str) -> Request<dto::AuthCredentials> {
	let credentials = dto::AuthCredentials {
		username: username.into(),
		password: password.into(),
	};
	Request::builder()
		.method(Method::POST)
		.uri("/api/auth")
		.body(credentials)
		.unwrap()
}

pub fn get_settings() -> Request<()> {
	Request::builder()
		.method(Method::GET)
		.uri("/api/settings")
		.body(())
		.unwrap()
}

pub fn put_settings(configuration: config::Config) -> Request<config::Config> {
	Request::builder()
		.method(Method::PUT)
		.uri("/api/settings")
		.body(configuration)
		.unwrap()
}

pub fn get_preferences() -> Request<()> {
	Request::builder()
		.method(Method::GET)
		.uri("/api/preferences")
		.body(())
		.unwrap()
}

pub fn put_preferences(preferences: config::Preferences) -> Request<config::Preferences> {
	Request::builder()
		.method(Method::PUT)
		.uri("/api/preferences")
		.body(preferences)
		.unwrap()
}

pub fn trigger_index() -> Request<()> {
	Request::builder()
		.method(Method::POST)
		.uri("/api/trigger_index")
		.body(())
		.unwrap()
}

pub fn browse(path: &Path) -> Request<()> {
	let path = path.to_string_lossy();
	let endpoint = format!("/api/browse/{}", url_encode(path.as_ref()));
	Request::builder()
		.method(Method::GET)
		.uri(&endpoint)
		.body(())
		.unwrap()
}

pub fn flatten(path: &Path) -> Request<()> {
	let path = path.to_string_lossy();
	let endpoint = format!("/api/flatten/{}", url_encode(path.as_ref()));
	Request::builder()
		.method(Method::GET)
		.uri(&endpoint)
		.body(())
		.unwrap()
}

pub fn random() -> Request<()> {
	Request::builder()
		.method(Method::GET)
		.uri("/api/random")
		.body(())
		.unwrap()
}

pub fn recent() -> Request<()> {
	Request::builder()
		.method(Method::GET)
		.uri("/api/recent")
		.body(())
		.unwrap()
}

pub fn search(query: &str) -> Request<()> {
	let endpoint = format!("/api/search/{}", url_encode(query));
	dbg!(&endpoint);
	Request::builder()
		.method(Method::GET)
		.uri(&endpoint)
		.body(())
		.unwrap()
}

pub fn audio(path: &Path) -> Request<()> {
	let path = path.to_string_lossy();
	let endpoint = format!("/api/audio/{}", url_encode(path.as_ref()));
	Request::builder()
		.method(Method::GET)
		.uri(&endpoint)
		.body(())
		.unwrap()
}

pub fn thumbnail(path: &Path, pad: Option<bool>) -> Request<()> {
	let path = path.to_string_lossy();
	let mut endpoint = format!("/api/thumbnail/{}", url_encode(path.as_ref()));
	match pad {
		Some(true) => endpoint.push_str("?pad=true"),
		Some(false) => endpoint.push_str("?pad=false"),
		None => (),
	};
	Request::builder()
		.method(Method::GET)
		.uri(&endpoint)
		.body(())
		.unwrap()
}

pub fn playlists() -> Request<()> {
	Request::builder()
		.method(Method::GET)
		.uri("/api/playlists")
		.body(())
		.unwrap()
}

pub fn save_playlist(
	name: &str,
	playlist: dto::SavePlaylistInput,
) -> Request<dto::SavePlaylistInput> {
	let endpoint = format!("/api/playlist/{}", url_encode(name));
	Request::builder()
		.method(Method::PUT)
		.uri(&endpoint)
		.body(playlist)
		.unwrap()
}

pub fn read_playlist(name: &str) -> Request<()> {
	let endpoint = format!("/api/playlist/{}", url_encode(name));
	Request::builder()
		.method(Method::GET)
		.uri(&endpoint)
		.body(())
		.unwrap()
}

pub fn delete_playlist(name: &str) -> Request<()> {
	let endpoint = format!("/api/playlist/{}", url_encode(name));
	Request::builder()
		.method(Method::DELETE)
		.uri(&endpoint)
		.body(())
		.unwrap()
}

fn url_encode(input: &str) -> String {
	percent_encode(input.as_bytes(), NON_ALPHANUMERIC).to_string()
}
