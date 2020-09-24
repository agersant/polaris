use cookie::Cookie;
use http::header::*;
use http::{HeaderMap, HeaderValue, Response, StatusCode};
use percent_encoding::{percent_encode, NON_ALPHANUMERIC};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::PathBuf;
use std::time::Duration;

use crate::service::constants::*;
use crate::service::dto;
use crate::{config, ddns, index, vfs};

#[cfg(feature = "service-rocket")]
pub use crate::service::rocket::test::ServiceType;

const TEST_DB_PREFIX: &str = "service-test-";
const TEST_USERNAME: &str = "test_user";
const TEST_PASSWORD: &str = "test_password";
const TEST_MOUNT_NAME: &str = "collection";
const TEST_MOUNT_SOURCE: &str = "test-data/small-collection";

pub trait TestService {
	fn new(db_name: &str) -> Self;
	fn get(&mut self, url: &str) -> Response<()>;
	fn get_bytes(&mut self, url: &str, headers: &HeaderMap<HeaderValue>) -> Response<Vec<u8>>;
	fn post(&mut self, url: &str) -> Response<()>;
	fn delete(&mut self, url: &str) -> Response<()>;
	fn get_json<T: DeserializeOwned>(&mut self, url: &str) -> Response<T>;
	fn put_json<T: Serialize>(&mut self, url: &str, payload: &T) -> Response<()>;
	fn post_json<T: Serialize>(&mut self, url: &str, payload: &T) -> Response<()>;

	fn complete_initial_setup(&mut self) {
		let configuration = config::Config {
			album_art_pattern: None,
			prefix_url: None,
			reindex_every_n_seconds: None,
			ydns: None,
			users: Some(vec![config::ConfigUser {
				name: TEST_USERNAME.into(),
				password: TEST_PASSWORD.into(),
				admin: true,
			}]),
			mount_dirs: Some(vec![vfs::MountPoint {
				name: TEST_MOUNT_NAME.into(),
				source: TEST_MOUNT_SOURCE.into(),
			}]),
		};
		self.put_json("/api/settings", &configuration);
	}

	fn login(&mut self) {
		let credentials = dto::AuthCredentials {
			username: TEST_USERNAME.into(),
			password: TEST_PASSWORD.into(),
		};
		self.post_json("/api/auth", &credentials);
	}

	fn index(&mut self) {
		assert!(self.post("/api/trigger_index").status() == StatusCode::OK);

		loop {
			let response = self.get_json::<Vec<index::CollectionFile>>("/api/browse");
			let entries = response.body();
			if entries.len() > 0 {
				break;
			}
			std::thread::sleep(Duration::from_secs(1));
		}

		loop {
			let response = self.get_json::<Vec<index::Song>>("/api/flatten");
			let entries = response.body();
			if entries.len() > 0 {
				break;
			}
			std::thread::sleep(Duration::from_secs(1));
		}
	}
}

#[test]
fn test_service_index() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.get("/");
}

#[test]
fn test_service_swagger_index() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	assert_eq!(
		service.get("/swagger").status(),
		StatusCode::PERMANENT_REDIRECT
	);
}

#[test]
fn test_service_swagger_index_with_trailing_slash() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	assert_eq!(service.get("/swagger/").status(), StatusCode::OK);
}

#[test]
fn test_service_version() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	let response = service.get_json::<dto::Version>("/api/version");
	let version = response.body();
	assert_eq!(version, &dto::Version { major: 5, minor: 0 });
}

#[test]
fn test_service_initial_setup() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	{
		let response = service.get_json::<dto::InitialSetup>("/api/initial_setup");
		let initial_setup = response.body();
		assert_eq!(
			initial_setup,
			&dto::InitialSetup {
				has_any_users: false
			}
		);
	}
	service.complete_initial_setup();
	{
		let response = service.get_json::<dto::InitialSetup>("/api/initial_setup");
		let initial_setup = response.body();
		assert_eq!(
			initial_setup,
			&dto::InitialSetup {
				has_any_users: true
			}
		);
	}
}

#[test]
fn test_service_settings() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();

	assert!(service.get("/api/settings").status() == StatusCode::UNAUTHORIZED);
	service.login();

	{
		let response = service.get_json::<config::Config>("/api/settings");
		let configuration = response.body();
		assert_eq!(
			configuration,
			&config::Config {
				album_art_pattern: Some("Folder.(jpg|png)".to_string()),
				reindex_every_n_seconds: Some(1800),
				mount_dirs: Some(vec![vfs::MountPoint {
					name: TEST_MOUNT_NAME.into(),
					source: TEST_MOUNT_SOURCE.into()
				}]),
				prefix_url: None,
				users: Some(vec![config::ConfigUser {
					name: TEST_USERNAME.into(),
					password: "".into(),
					admin: true
				}]),
				ydns: Some(ddns::DDNSConfig {
					host: "".into(),
					username: "".into(),
					password: "".into()
				}),
			}
		);
	}

	let mut configuration = config::Config {
		album_art_pattern: Some("my_pattern".to_owned()),
		reindex_every_n_seconds: Some(3600),
		mount_dirs: Some(vec![
			vfs::MountPoint {
				name: TEST_MOUNT_NAME.into(),
				source: TEST_MOUNT_SOURCE.into(),
			},
			vfs::MountPoint {
				name: "more_music".into(),
				source: "test-data/small-collection".into(),
			},
		]),
		prefix_url: Some("my_prefix".to_owned()),
		users: Some(vec![
			config::ConfigUser {
				name: "test_user".into(),
				password: "some_password".into(),
				admin: false,
			},
			config::ConfigUser {
				name: "other_user".into(),
				password: "some_other_password".into(),
				admin: false,
			},
		]),
		ydns: Some(ddns::DDNSConfig {
			host: "my_host".into(),
			username: "my_username".into(),
			password: "my_password".into(),
		}),
	};

	service.put_json("/api/settings", &configuration);

	configuration.users = Some(vec![
		config::ConfigUser {
			name: "test_user".into(),
			password: "".into(),
			admin: true,
		},
		config::ConfigUser {
			name: "other_user".into(),
			password: "".into(),
			admin: false,
		},
	]);

	let response = service.get_json::<config::Config>("/api/settings");
	let received = response.body();
	assert_eq!(received, &configuration);
}

#[test]
fn test_service_preferences() {
	// TODO
}

#[test]
fn test_service_trigger_index() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();

	let response = service.get_json::<Vec<index::Directory>>("/api/random");
	let entries = response.body();
	assert_eq!(entries.len(), 0);

	service.index();

	let response = service.get_json::<Vec<index::Directory>>("/api/random");
	let entries = response.body();
	assert_eq!(entries.len(), 3);
}

#[test]
fn test_service_auth() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();

	{
		let credentials = dto::AuthCredentials {
			username: "garbage".into(),
			password: "garbage".into(),
		};
		assert!(service.post_json("/api/auth", &credentials).status() == StatusCode::UNAUTHORIZED);
	}
	{
		let credentials = dto::AuthCredentials {
			username: TEST_USERNAME.into(),
			password: "garbage".into(),
		};
		assert!(service.post_json("/api/auth", &credentials).status() == StatusCode::UNAUTHORIZED);
	}
	{
		let credentials = dto::AuthCredentials {
			username: TEST_USERNAME.into(),
			password: TEST_PASSWORD.into(),
		};
		let response = service.post_json("/api/auth", &credentials);
		assert!(response.status() == StatusCode::OK);
		let cookies: Vec<Cookie> = response
			.headers()
			.get_all(SET_COOKIE)
			.iter()
			.map(|c| Cookie::parse(c.to_str().unwrap()).unwrap())
			.collect();
		assert!(cookies.iter().any(|c| c.name() == COOKIE_SESSION));
		assert!(cookies.iter().any(|c| c.name() == COOKIE_USERNAME));
		assert!(cookies.iter().any(|c| c.name() == COOKIE_ADMIN));
	}
}

#[test]
fn test_service_browse() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();
	service.index();

	let response = service.get_json::<Vec<index::CollectionFile>>("/api/browse");
	let entries = response.body();
	assert_eq!(entries.len(), 1);

	let mut path = PathBuf::new();
	path.push("collection");
	path.push("Khemmis");
	path.push("Hunted");
	let uri = format!(
		"/api/browse/{}",
		percent_encode(path.to_string_lossy().as_ref().as_bytes(), NON_ALPHANUMERIC)
	);

	let response = service.get_json::<Vec<index::CollectionFile>>(&uri);
	let entries = response.body();
	assert_eq!(entries.len(), 5);
}

#[test]
fn test_service_flatten() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();
	service.index();

	let response = service.get_json::<Vec<index::Song>>("/api/flatten");
	let entries = response.body();
	assert_eq!(entries.len(), 13);

	let response = service.get_json::<Vec<index::Song>>("/api/flatten/collection");
	let entries = response.body();
	assert_eq!(entries.len(), 13);
}

#[test]
fn test_service_random() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();
	service.index();

	let response = service.get_json::<Vec<index::Directory>>("/api/random");
	let entries = response.body();
	assert_eq!(entries.len(), 3);
}

#[test]
fn test_service_recent() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();
	service.index();

	let response = service.get_json::<Vec<index::Directory>>("/api/recent");
	let entries = response.body();
	assert_eq!(entries.len(), 3);
}

#[test]
fn test_service_search() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();
	service.index();

	let response = service.get_json::<Vec<index::CollectionFile>>("/api/search/door");
	let results = response.body();
	assert_eq!(results.len(), 1);
	match results[0] {
		index::CollectionFile::Song(ref s) => assert_eq!(s.title, Some("Beyond The Door".into())),
		_ => panic!(),
	}
}

#[test]
fn test_service_serve() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();
	service.index();

	let mut path = PathBuf::new();
	path.push("collection");
	path.push("Khemmis");
	path.push("Hunted");
	path.push("02 - Candlelight.mp3");
	let uri = format!(
		"/api/audio/{}",
		percent_encode(path.to_string_lossy().as_ref().as_bytes(), NON_ALPHANUMERIC)
	);

	let response = service.get_bytes(&uri, &HeaderMap::new());
	assert_eq!(response.status(), StatusCode::OK);
	assert_eq!(response.body().len(), 24_142);

	{
		let mut headers = HeaderMap::new();
		headers.append(RANGE, HeaderValue::from_str("bytes=100-299").unwrap());
		let response = service.get_bytes(&uri, &headers);
		assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
		assert_eq!(response.body().len(), 200);
		assert_eq!(response.headers().get(CONTENT_LENGTH).unwrap(), "200");
	}
}

#[test]
fn test_service_playlists() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();
	service.index();

	let response = service.get_json::<Vec<dto::ListPlaylistsEntry>>("/api/playlists");
	let playlists = response.body();
	assert_eq!(playlists.len(), 0);

	let response = service.get_json::<Vec<index::Song>>("/api/flatten");
	let mut my_songs = response.into_body();
	my_songs.pop();
	my_songs.pop();
	let my_playlist = dto::SavePlaylistInput {
		tracks: my_songs.iter().map(|s| s.path.clone()).collect(),
	};
	service.put_json("/api/playlist/my_playlist", &my_playlist);

	let response = service.get_json::<Vec<dto::ListPlaylistsEntry>>("/api/playlists");
	let playlists = response.body();
	assert_eq!(
		playlists,
		&vec![dto::ListPlaylistsEntry {
			name: "my_playlist".into()
		}]
	);

	let response = service.get_json::<Vec<index::Song>>("/api/playlist/my_playlist");
	let songs = response.body();
	assert_eq!(songs, &my_songs);

	service.delete("/api/playlist/my_playlist");

	let response = service.get_json::<Vec<dto::ListPlaylistsEntry>>("/api/playlists");
	let playlists = response.body();
	assert_eq!(playlists.len(), 0);
}

#[test]
fn test_service_thumbnail() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();
	service.index();

	let mut path = PathBuf::new();
	path.push("collection");
	path.push("Khemmis");
	path.push("Hunted");
	path.push("Folder.jpg");
	let uri = format!(
		"/api/thumbnail/{}",
		percent_encode(path.to_string_lossy().as_ref().as_bytes(), NON_ALPHANUMERIC)
	);

	let response = service.get(&uri);
	assert_eq!(response.status(), StatusCode::OK);
}
