use function_name::named;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::PathBuf;
use std::time::Duration;
use url::form_urlencoded::byte_serialize;

use crate::service::dto;
use crate::{config, ddns, index, vfs};

#[cfg(feature = "service-rocket")]
pub use crate::service::rocket::test::ServiceType;

const TEST_USERNAME: &str = "test_user";
const TEST_PASSWORD: &str = "test_password";
const TEST_MOUNT_NAME: &str = "collection";
const TEST_MOUNT_SOURCE: &str = "test/collection";

pub trait HttpStatus {
	fn is_ok(&self) -> bool;
	fn is_unauthorized(&self) -> bool;
}

pub trait TestService {
	type Status: HttpStatus;

	fn new(db_name: &str) -> Self;
	fn get(&mut self, url: &str) -> Self::Status;
	fn post(&mut self, url: &str) -> Self::Status;
	fn delete(&mut self, url: &str) -> Self::Status;
	fn get_json<T: DeserializeOwned>(&mut self, url: &str) -> T;
	fn put_json<T: Serialize>(&mut self, url: &str, payload: &T);
	fn post_json<T: Serialize>(&mut self, url: &str, payload: &T) -> Self::Status;

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
		assert!(self.post("/api/trigger_index").is_ok());
		for _ in 1..20 {
			let entries: Vec<index::CollectionFile> = self.get_json("/api/browse");
			if entries.len() > 0 {
				return;
			}
			std::thread::sleep(Duration::from_secs(1));
		}
		panic!("index timeout");
	}
}

#[named]
#[test]
fn test_service_index() {
	let mut service = ServiceType::new(function_name!());
	service.get("/");
}

#[named]
#[test]
fn test_service_swagger_index() {
	let mut service = ServiceType::new(function_name!());
	assert!(service.get("/swagger").is_ok());
}

#[named]
#[test]
fn test_service_swagger_index_with_trailing_slash() {
	let mut service = ServiceType::new(function_name!());
	assert!(service.get("/swagger/").is_ok());
}

#[named]
#[test]
fn test_service_version() {
	let mut service = ServiceType::new(function_name!());
	let version: dto::Version = service.get_json("/api/version");
	assert_eq!(version, dto::Version { major: 4, minor: 0 });
}

#[named]
#[test]
fn test_service_initial_setup() {
	let mut service = ServiceType::new(function_name!());
	{
		let initial_setup: dto::InitialSetup = service.get_json("/api/initial_setup");
		assert_eq!(
			initial_setup,
			dto::InitialSetup {
				has_any_users: false
			}
		);
	}
	service.complete_initial_setup();
	{
		let initial_setup: dto::InitialSetup = service.get_json("/api/initial_setup");
		assert_eq!(
			initial_setup,
			dto::InitialSetup {
				has_any_users: true
			}
		);
	}
}

#[named]
#[test]
fn test_service_settings() {
	let mut service = ServiceType::new(function_name!());
	service.complete_initial_setup();

	assert!(service.get("/api/settings").is_unauthorized());
	service.login();

	{
		let configuration: config::Config = service.get_json("/api/settings");
		assert_eq!(
			configuration,
			config::Config {
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
				source: "test/collection".into(),
			},
		]),
		prefix_url: Some("my_prefix".to_owned()),
		users: Some(vec![
			config::ConfigUser {
				name: "test_user".into(),
				password: "some_password".into(),
				admin: true,
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

	let received: config::Config = service.get_json("/api/settings");
	assert_eq!(received, configuration);
}

#[named]
#[test]
fn test_service_preferences() {
	// TODO
}

#[named]
#[test]
fn test_service_trigger_index() {
	let mut service = ServiceType::new(function_name!());
	service.complete_initial_setup();
	service.login();

	let entries: Vec<index::Directory> = service.get_json("/api/random");
	assert_eq!(entries.len(), 0);

	service.index();

	let entries: Vec<index::Directory> = service.get_json("/api/random");
	assert_eq!(entries.len(), 2);
}

#[named]
#[test]
fn test_service_auth() {
	let mut service = ServiceType::new(function_name!());
	service.complete_initial_setup();

	{
		let credentials = dto::AuthCredentials {
			username: "garbage".into(),
			password: "garbage".into(),
		};
		assert!(service
			.post_json("/api/auth", &credentials)
			.is_unauthorized());
	}
	{
		let credentials = dto::AuthCredentials {
			username: TEST_USERNAME.into(),
			password: "garbage".into(),
		};
		assert!(service
			.post_json("/api/auth", &credentials)
			.is_unauthorized());
	}
	{
		let credentials = dto::AuthCredentials {
			username: TEST_USERNAME.into(),
			password: TEST_PASSWORD.into(),
		};
		assert!(service.post_json("/api/auth", &credentials).is_ok());
		// TODO validate cookies
	}
}

#[named]
#[test]
fn test_service_browse() {
	let mut service = ServiceType::new(function_name!());
	service.complete_initial_setup();
	service.login();
	service.index();

	let entries: Vec<index::CollectionFile> = service.get_json("/api/browse");
	assert_eq!(entries.len(), 1);

	let mut path = PathBuf::new();
	path.push("collection");
	path.push("Khemmis");
	path.push("Hunted");
	let encoded_path: String = byte_serialize(path.to_string_lossy().as_ref().as_bytes()).collect();
	let uri = format!("/api/browse/{}", encoded_path);

	let entries: Vec<index::CollectionFile> = service.get_json(&uri);
	assert_eq!(entries.len(), 5);
}

#[named]
#[test]
fn test_service_flatten() {
	let mut service = ServiceType::new(function_name!());
	service.complete_initial_setup();
	service.login();
	service.index();

	let entries: Vec<index::Song> = service.get_json("/api/flatten");
	assert_eq!(entries.len(), 12);

	let entries: Vec<index::Song> = service.get_json("/api/flatten/collection");
	assert_eq!(entries.len(), 12);
}

#[named]
#[test]
fn test_service_random() {
	let mut service = ServiceType::new(function_name!());
	service.complete_initial_setup();
	service.login();
	service.index();

	let entries: Vec<index::Directory> = service.get_json("/api/random");
	assert_eq!(entries.len(), 2);
}

#[named]
#[test]
fn test_service_recent() {
	let mut service = ServiceType::new(function_name!());
	service.complete_initial_setup();
	service.login();
	service.index();

	let entries: Vec<index::Directory> = service.get_json("/api/recent");
	assert_eq!(entries.len(), 2);
}

#[named]
#[test]
fn test_service_search() {
	let mut service = ServiceType::new(function_name!());
	service.complete_initial_setup();
	service.login();
	service.index();

	let results: Vec<index::CollectionFile> = service.get_json("/api/search/door");
	assert_eq!(results.len(), 1);
	match results[0] {
		index::CollectionFile::Song(ref s) => assert_eq!(s.title, Some("Beyond The Door".into())),
		_ => panic!(),
	}
}

/* TODO
#[named]
#[test]
fn test_service_serve() {
	let mut service = ServiceType::new(function_name!());
	service.complete_initial_setup();
	service.login();
	service.index();

	{
		let mut response = client
			.get("/api/serve/collection%2FKhemmis%2FHunted%2F02%20-%20Candlelight.mp3")
			.dispatch();
		assert_eq!(response.status(), Status::Ok);
		let body = response.body().unwrap();
		let body = body.into_bytes().unwrap();
		assert_eq!(body.len(), 24_142);
	}

	{
		let mut response = client
			.get("/api/serve/collection%2FKhemmis%2FHunted%2F02%20-%20Candlelight.mp3")
			.header(Range::bytes(100, 299))
			.dispatch();
		assert_eq!(response.status(), Status::PartialContent);
		let body = response.body().unwrap();
		let body = body.into_bytes().unwrap();
		assert_eq!(body.len(), 200);
		assert_eq!(response.headers().get_one("Content-Length").unwrap(), "200");
	}
}
*/

#[named]
#[test]
fn test_service_playlists() {
	let mut service = ServiceType::new(function_name!());
	service.complete_initial_setup();
	service.login();
	service.index();

	let playlists: Vec<dto::ListPlaylistsEntry> = service.get_json("/api/playlists");
	assert_eq!(playlists.len(), 0);

	let mut my_songs: Vec<index::Song> = service.get_json("/api/flatten");
	my_songs.pop();
	my_songs.pop();
	let my_playlist = dto::SavePlaylistInput {
		tracks: my_songs.iter().map(|s| s.path.clone()).collect(),
	};
	service.put_json("/api/playlist/my_playlist", &my_playlist);

	let playlists: Vec<dto::ListPlaylistsEntry> = service.get_json("/api/playlists");
	assert_eq!(
		playlists,
		vec![dto::ListPlaylistsEntry {
			name: "my_playlist".into()
		}]
	);

	let songs: Vec<index::Song> = service.get_json("/api/playlist/my_playlist");
	assert_eq!(songs, my_songs);

	service.delete("/api/playlist/my_playlist");

	let playlists: Vec<dto::ListPlaylistsEntry> = service.get_json("/api/playlists");
	assert_eq!(playlists.len(), 0);
}
