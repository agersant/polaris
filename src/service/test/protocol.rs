use http::{method::Method, Request};
use percent_encoding::{percent_encode, NON_ALPHANUMERIC};
use std::path::Path;

use crate::config;
use crate::service::dto;

pub struct RequestBuilder {
	prefix: String,
}

impl RequestBuilder {
	pub fn new(prefix: String) -> Self {
		Self { prefix }
	}

	fn build_uri(&self, endpoint: &str) -> String {
		format!("{}{}", self.prefix, endpoint)
	}

	pub fn web_index(&self) -> Request<()> {
		Request::builder()
			.method(Method::GET)
			.uri(self.build_uri("/"))
			.body(())
			.unwrap()
	}

	pub fn swagger_index(&self) -> Request<()> {
		Request::builder()
			.method(Method::GET)
			.uri(self.build_uri("/swagger/"))
			.body(())
			.unwrap()
	}

	pub fn version(&self) -> Request<()> {
		Request::builder()
			.method(Method::GET)
			.uri(self.build_uri("/api/version"))
			.body(())
			.unwrap()
	}

	pub fn initial_setup(&self) -> Request<()> {
		Request::builder()
			.method(Method::GET)
			.uri(self.build_uri("/api/initial_setup"))
			.body(())
			.unwrap()
	}

	pub fn login(&self, username: &str, password: &str) -> Request<dto::AuthCredentials> {
		let credentials = dto::AuthCredentials {
			username: username.into(),
			password: password.into(),
		};
		Request::builder()
			.method(Method::POST)
			.uri(self.build_uri("/api/auth"))
			.body(credentials)
			.unwrap()
	}

	pub fn get_settings(&self) -> Request<()> {
		Request::builder()
			.method(Method::GET)
			.uri(self.build_uri("/api/settings"))
			.body(())
			.unwrap()
	}

	pub fn put_settings(&self, configuration: config::Config) -> Request<config::Config> {
		Request::builder()
			.method(Method::PUT)
			.uri(self.build_uri("/api/settings"))
			.body(configuration)
			.unwrap()
	}

	pub fn get_preferences(&self) -> Request<()> {
		Request::builder()
			.method(Method::GET)
			.uri(self.build_uri("/api/preferences"))
			.body(())
			.unwrap()
	}

	pub fn put_preferences(
		&self,
		preferences: config::Preferences,
	) -> Request<config::Preferences> {
		Request::builder()
			.method(Method::PUT)
			.uri(self.build_uri("/api/preferences"))
			.body(preferences)
			.unwrap()
	}

	pub fn trigger_index(&self) -> Request<()> {
		Request::builder()
			.method(Method::POST)
			.uri(self.build_uri("/api/trigger_index"))
			.body(())
			.unwrap()
	}

	pub fn browse(&self, path: &Path) -> Request<()> {
		let path = path.to_string_lossy();
		let endpoint = format!("/api/browse/{}", url_encode(path.as_ref()));
		Request::builder()
			.method(Method::GET)
			.uri(self.build_uri(&endpoint))
			.body(())
			.unwrap()
	}

	pub fn flatten(&self, path: &Path) -> Request<()> {
		let path = path.to_string_lossy();
		let endpoint = format!("/api/flatten/{}", url_encode(path.as_ref()));
		Request::builder()
			.method(Method::GET)
			.uri(self.build_uri(&endpoint))
			.body(())
			.unwrap()
	}

	pub fn random(&self) -> Request<()> {
		Request::builder()
			.method(Method::GET)
			.uri(self.build_uri("/api/random"))
			.body(())
			.unwrap()
	}

	pub fn recent(&self) -> Request<()> {
		Request::builder()
			.method(Method::GET)
			.uri(self.build_uri("/api/recent"))
			.body(())
			.unwrap()
	}

	pub fn search(&self, query: &str) -> Request<()> {
		let endpoint = format!("/api/search/{}", url_encode(query));
		dbg!(self.build_uri(&endpoint));
		Request::builder()
			.method(Method::GET)
			.uri(self.build_uri(&endpoint))
			.body(())
			.unwrap()
	}

	pub fn audio(&self, path: &Path) -> Request<()> {
		let path = path.to_string_lossy();
		let endpoint = format!("/api/audio/{}", url_encode(path.as_ref()));
		Request::builder()
			.method(Method::GET)
			.uri(self.build_uri(&endpoint))
			.body(())
			.unwrap()
	}

	pub fn thumbnail(&self, path: &Path, pad: Option<bool>) -> Request<()> {
		let path = path.to_string_lossy();
		let mut endpoint = format!("/api/thumbnail/{}", url_encode(path.as_ref()));
		match pad {
			Some(true) => endpoint.push_str("?pad=true"),
			Some(false) => endpoint.push_str("?pad=false"),
			None => (),
		};
		Request::builder()
			.method(Method::GET)
			.uri(self.build_uri(&endpoint))
			.body(())
			.unwrap()
	}

	pub fn playlists(&self) -> Request<()> {
		Request::builder()
			.method(Method::GET)
			.uri(self.build_uri("/api/playlists"))
			.body(())
			.unwrap()
	}

	pub fn save_playlist(
		&self,
		name: &str,
		playlist: dto::SavePlaylistInput,
	) -> Request<dto::SavePlaylistInput> {
		let endpoint = format!("/api/playlist/{}", url_encode(name));
		Request::builder()
			.method(Method::PUT)
			.uri(self.build_uri(&endpoint))
			.body(playlist)
			.unwrap()
	}

	pub fn read_playlist(&self, name: &str) -> Request<()> {
		let endpoint = format!("/api/playlist/{}", url_encode(name));
		Request::builder()
			.method(Method::GET)
			.uri(self.build_uri(&endpoint))
			.body(())
			.unwrap()
	}

	pub fn delete_playlist(&self, name: &str) -> Request<()> {
		let endpoint = format!("/api/playlist/{}", url_encode(name));
		Request::builder()
			.method(Method::DELETE)
			.uri(self.build_uri(&endpoint))
			.body(())
			.unwrap()
	}
}

fn url_encode(input: &str) -> String {
	percent_encode(input.as_bytes(), NON_ALPHANUMERIC).to_string()
}
