use http::{Method, Request};
use percent_encoding::{percent_encode, NON_ALPHANUMERIC};
use std::path::Path;

use crate::service::dto;
use crate::{app::user, service::dto::ThumbnailSize};

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

pub fn apply_config(config: dto::Config) -> Request<dto::Config> {
	Request::builder()
		.method(Method::PUT)
		.uri("/api/config")
		.body(config)
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

pub fn get_ddns_config() -> Request<()> {
	Request::builder()
		.method(Method::GET)
		.uri("/api/ddns")
		.body(())
		.unwrap()
}

pub fn put_ddns_config(ddns_config: dto::DDNSConfig) -> Request<dto::DDNSConfig> {
	Request::builder()
		.method(Method::PUT)
		.uri("/api/ddns")
		.body(ddns_config)
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

pub fn get_preferences() -> Request<()> {
	Request::builder()
		.method(Method::GET)
		.uri("/api/preferences")
		.body(())
		.unwrap()
}

pub fn put_preferences(preferences: user::Preferences) -> Request<user::Preferences> {
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

pub fn thumbnail(path: &Path, size: Option<ThumbnailSize>, pad: Option<bool>) -> Request<()> {
	let path = path.to_string_lossy();
	let mut params = String::new();
	if let Some(s) = size {
		params.push('?');
		match s {
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

pub fn lastfm_link_token() -> Request<()> {
	Request::builder()
		.method(Method::GET)
		.uri("/api/lastfm/link_token")
		.body(())
		.unwrap()
}

pub fn lastfm_now_playing(path: &Path) -> Request<()> {
	let path = path.to_string_lossy();
	let endpoint = format!("/api/lastfm/now_playing/{}", url_encode(path.as_ref()));
	Request::builder()
		.method(Method::PUT)
		.uri(&endpoint)
		.body(())
		.unwrap()
}

pub fn lastfm_scrobble(path: &Path) -> Request<()> {
	let path = path.to_string_lossy();
	let endpoint = format!("/api/lastfm/scrobble/{}", url_encode(path.as_ref()));
	Request::builder()
		.method(Method::POST)
		.uri(&endpoint)
		.body(())
		.unwrap()
}

fn url_encode(input: &str) -> String {
	percent_encode(input.as_bytes(), NON_ALPHANUMERIC).to_string()
}
