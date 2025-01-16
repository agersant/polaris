use http::{Method, Request};
use percent_encoding::{percent_encode, NON_ALPHANUMERIC};
use std::path::Path;

use crate::server::dto;
use crate::server::dto::ThumbnailSize;

pub trait ProtocolVersion {
	fn header_value() -> i32;
}

pub struct V7;
pub struct V8;

impl ProtocolVersion for V7 {
	fn header_value() -> i32 {
		7
	}
}

impl ProtocolVersion for V8 {
	fn header_value() -> i32 {
		8
	}
}

pub fn web_index() -> Request<()> {
	Request::builder()
		.method(Method::GET)
		.uri("/")
		.body(())
		.unwrap()
}

pub fn docs_index() -> Request<()> {
	Request::builder()
		.method(Method::GET)
		.uri("/api-docs")
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

pub fn login(username: &str, password: &str) -> Request<dto::Credentials> {
	let credentials = dto::Credentials {
		username: username.into(),
		password: password.into(),
	};
	Request::builder()
		.method(Method::POST)
		.uri("/api/auth")
		.body(credentials)
		.unwrap()
}

pub fn put_mount_dirs(dirs: Vec<dto::MountDir>) -> Request<Vec<dto::MountDir>> {
	Request::builder()
		.method(Method::PUT)
		.uri("/api/mount_dirs")
		.body(dirs)
		.unwrap()
}

pub fn get_settings() -> Request<()> {
	Request::builder()
		.method(Method::GET)
		.uri("/api/settings")
		.body(())
		.unwrap()
}

pub fn put_settings(settings: dto::NewSettings) -> Request<dto::NewSettings> {
	Request::builder()
		.method(Method::PUT)
		.uri("/api/settings")
		.body(settings)
		.unwrap()
}

pub fn list_users() -> Request<()> {
	Request::builder()
		.method(Method::GET)
		.uri("/api/users")
		.body(())
		.unwrap()
}

pub fn create_user(new_user: dto::NewUser) -> Request<dto::NewUser> {
	Request::builder()
		.method(Method::POST)
		.uri("/api/user")
		.body(new_user)
		.unwrap()
}

pub fn update_user(username: &str, user_update: dto::UserUpdate) -> Request<dto::UserUpdate> {
	Request::builder()
		.method(Method::PUT)
		.uri(format!("/api/user/{}", username))
		.body(user_update)
		.unwrap()
}

pub fn delete_user(username: &str) -> Request<()> {
	Request::builder()
		.method(Method::DELETE)
		.uri(format!("/api/user/{}", username))
		.body(())
		.unwrap()
}

pub fn trigger_index() -> Request<()> {
	Request::builder()
		.method(Method::POST)
		.uri("/api/trigger_index")
		.body(())
		.unwrap()
}

pub fn browse<VERSION: ProtocolVersion>(path: &Path) -> Request<()> {
	let path = path.to_string_lossy();
	let endpoint = format!("/api/browse/{}", url_encode(path.as_ref()));
	Request::builder()
		.header("Accept-Version", VERSION::header_value())
		.method(Method::GET)
		.uri(&endpoint)
		.body(())
		.unwrap()
}

pub fn flatten<VERSION: ProtocolVersion>(path: &Path) -> Request<()> {
	let path = path.to_string_lossy();
	let endpoint = format!("/api/flatten/{}", url_encode(path.as_ref()));
	Request::builder()
		.header("Accept-Version", VERSION::header_value())
		.method(Method::GET)
		.uri(&endpoint)
		.body(())
		.unwrap()
}

pub fn genres<VERSION: ProtocolVersion>() -> Request<()> {
	Request::builder()
		.header("Accept-Version", VERSION::header_value())
		.method(Method::GET)
		.uri("/api/genres")
		.body(())
		.unwrap()
}

pub fn genre<VERSION: ProtocolVersion>(genre: &str) -> Request<()> {
	let endpoint = format!("/api/genre/{}", url_encode(genre));
	Request::builder()
		.header("Accept-Version", VERSION::header_value())
		.method(Method::GET)
		.uri(endpoint)
		.body(())
		.unwrap()
}

pub fn genre_albums<VERSION: ProtocolVersion>(genre: &str) -> Request<()> {
	let endpoint = format!("/api/genre/{}/albums", url_encode(genre));
	Request::builder()
		.header("Accept-Version", VERSION::header_value())
		.method(Method::GET)
		.uri(endpoint)
		.body(())
		.unwrap()
}

pub fn genre_artists<VERSION: ProtocolVersion>(genre: &str) -> Request<()> {
	let endpoint = format!("/api/genre/{}/artists", url_encode(genre));
	Request::builder()
		.header("Accept-Version", VERSION::header_value())
		.method(Method::GET)
		.uri(endpoint)
		.body(())
		.unwrap()
}

pub fn genre_songs<VERSION: ProtocolVersion>(genre: &str) -> Request<()> {
	let endpoint = format!("/api/genre/{}/songs", url_encode(genre));
	Request::builder()
		.header("Accept-Version", VERSION::header_value())
		.method(Method::GET)
		.uri(endpoint)
		.body(())
		.unwrap()
}

pub fn random<VERSION: ProtocolVersion>() -> Request<()> {
	Request::builder()
		.header("Accept-Version", VERSION::header_value())
		.method(Method::GET)
		.uri("/api/albums/random")
		.body(())
		.unwrap()
}

pub fn recent<VERSION: ProtocolVersion>() -> Request<()> {
	Request::builder()
		.header("Accept-Version", VERSION::header_value())
		.method(Method::GET)
		.uri("/api/albums/recent")
		.body(())
		.unwrap()
}

pub fn search<VERSION: ProtocolVersion>(query: &str) -> Request<()> {
	let endpoint = format!("/api/search/{}", url_encode(query));
	Request::builder()
		.header("Accept-Version", VERSION::header_value())
		.method(Method::GET)
		.uri(&endpoint)
		.body(())
		.unwrap()
}

pub fn songs(songs: dto::GetSongsBulkInput) -> Request<dto::GetSongsBulkInput> {
	Request::builder()
		.method(Method::POST)
		.uri("/api/songs")
		.body(songs)
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

pub fn peaks(path: &Path) -> Request<()> {
	let path = path.to_string_lossy();
	let endpoint = format!("/api/peaks/{}", url_encode(path.as_ref()));
	Request::builder()
		.method(Method::GET)
		.uri(&endpoint)
		.body(())
		.unwrap()
}

pub fn thumbnail(path: &Path, size: Option<ThumbnailSize>, pad: Option<bool>) -> Request<()> {
	let path = path.to_string_lossy();
	let mut params = String::new();
	if let Some(s) = size {
		params.push('?');
		match s {
			ThumbnailSize::Tiny => params.push_str("size=tiny"),
			ThumbnailSize::Small => params.push_str("size=small"),
			ThumbnailSize::Large => params.push_str("size=large"),
			ThumbnailSize::Native => params.push_str("size=native"),
		};
	}
	if let Some(p) = pad {
		if params.is_empty() {
			params.push('?');
		} else {
			params.push('&');
		}
		if p {
			params.push_str("pad=true");
		} else {
			params.push_str("pad=false");
		};
	}

	let endpoint = format!("/api/thumbnail/{}{}", url_encode(path.as_ref()), params);

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

pub fn read_playlist<VERSION: ProtocolVersion>(name: &str) -> Request<()> {
	let endpoint = format!("/api/playlist/{}", url_encode(name));
	Request::builder()
		.header("Accept-Version", VERSION::header_value())
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
