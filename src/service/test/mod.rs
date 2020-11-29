use cookie::Cookie;
use http::header::*;
use http::{HeaderValue, Request, Response, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub mod protocol;

use crate::service::constants::*;
use crate::service::dto;
use crate::{config, index, vfs};

#[cfg(feature = "service-rocket")]
pub use crate::service::rocket::test::ServiceType;

const TEST_DB_PREFIX: &str = "service-test-";
const TEST_USERNAME: &str = "test_user";
const TEST_PASSWORD: &str = "test_password";
const TEST_MOUNT_NAME: &str = "collection";
const TEST_MOUNT_SOURCE: &str = "test-data/small-collection";

pub struct GenericPayload {
	pub content_type: Option<&'static str>,
	pub content: Option<Vec<u8>>,
}

pub trait Payload {
	fn send(&self) -> GenericPayload;
}

impl<T: Serialize> Payload for T {
	fn send(&self) -> GenericPayload {
		GenericPayload {
			content_type: Some("application/json"),
			content: Some(serde_json::to_string(self).unwrap().as_bytes().into()),
		}
	}
}

pub trait TestService {
	fn new(db_name: &str) -> Self;
	fn request_builder(&self) -> &protocol::RequestBuilder;
	fn process<T: Payload>(&mut self, request: &Request<T>) -> Response<()>;
	fn fetch_bytes<T: Payload>(&mut self, request: &Request<T>) -> Response<Vec<u8>>;
	fn fetch_json<T: Payload, U: DeserializeOwned>(&mut self, request: &Request<T>) -> Response<U>;

	fn complete_initial_setup(&mut self) {
		let configuration = config::Config {
			album_art_pattern: None,
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
		let request = self.request_builder().put_settings(configuration);
		let response = self.process(&request);
		assert_eq!(response.status(), StatusCode::OK);
	}

	fn login(&mut self) {
		let request = self.request_builder().login(TEST_USERNAME, TEST_PASSWORD);
		let response = self.process(&request);
		assert_eq!(response.status(), StatusCode::OK);
	}

	fn index(&mut self) {
		let request = self.request_builder().trigger_index();
		let response = self.process(&request);
		assert_eq!(response.status(), StatusCode::OK);

		loop {
			let browse_request = self.request_builder().browse(Path::new(""));
			let response = self.fetch_json::<(), Vec<index::CollectionFile>>(&browse_request);
			let entries = response.body();
			if entries.len() > 0 {
				break;
			}
			std::thread::sleep(Duration::from_secs(1));
		}

		loop {
			let flatten_request = self.request_builder().flatten(Path::new(""));
			let response = self.fetch_json::<_, Vec<index::Song>>(&flatten_request);
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
	let request = service.request_builder().web_index();
	let _response = service.fetch_bytes(&request);
}

#[test]
fn test_service_swagger_index() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	let request = service.request_builder().swagger_index();
	let response = service.fetch_bytes(&request);
	assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn test_service_version() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	let request = service.request_builder().version();
	let response = service.fetch_json::<_, dto::Version>(&request);
	assert_eq!(response.status(), StatusCode::OK);
	let version = response.body();
	assert_eq!(version, &dto::Version { major: 5, minor: 0 });
}

#[test]
fn test_service_initial_setup() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	let request = service.request_builder().initial_setup();
	{
		let response = service.fetch_json::<_, dto::InitialSetup>(&request);
		assert_eq!(response.status(), StatusCode::OK);
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
		let response = service.fetch_json::<_, dto::InitialSetup>(&request);
		assert_eq!(response.status(), StatusCode::OK);
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

	let get_settings = service.request_builder().get_settings();

	let response = service.process(&get_settings);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

	service.login();

	let response = service.fetch_json::<_, config::Config>(&get_settings);
	assert_eq!(response.status(), StatusCode::OK);

	let put_settings = service
		.request_builder()
		.put_settings(config::Config::default());
	let response = service.process(&put_settings);
	assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn test_service_settings_cannot_unadmin_self() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();

	let mut configuration = config::Config::default();
	configuration.users = Some(vec![config::ConfigUser {
		name: TEST_USERNAME.into(),
		password: "".into(),
		admin: false,
	}]);
	let request = service.request_builder().put_settings(configuration);
	let response = service.process(&request);
	assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[test]
fn test_service_preferences() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();

	let request = service.request_builder().get_preferences();
	let response = service.fetch_json::<_, config::Preferences>(&request);
	assert_eq!(response.status(), StatusCode::OK);

	let request = service
		.request_builder()
		.put_preferences(config::Preferences::default());
	let response = service.process(&request);
	assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn test_service_trigger_index() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();

	let request = service.request_builder().random();

	let response = service.fetch_json::<_, Vec<index::Directory>>(&request);
	let entries = response.body();
	assert_eq!(entries.len(), 0);

	service.index();

	let response = service.fetch_json::<_, Vec<index::Directory>>(&request);
	let entries = response.body();
	assert_eq!(entries.len(), 3);
}

#[test]
fn test_service_auth() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();

	{
		let request = service.request_builder().login("garbage", "garbage");
		let response = service.process(&request);
		assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
	}
	{
		let request = service.request_builder().login(TEST_USERNAME, "garbage");
		let response = service.process(&request);
		assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
	}
	{
		let request = service
			.request_builder()
			.login(TEST_USERNAME, TEST_PASSWORD);
		let response = service.process(&request);
		assert_eq!(response.status(), StatusCode::OK);

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

	{
		let request = service.request_builder().browse(&PathBuf::new());
		let response = service.fetch_json::<_, Vec<index::CollectionFile>>(&request);
		assert_eq!(response.status(), StatusCode::OK);
		let entries = response.body();
		assert_eq!(entries.len(), 1);
	}

	{
		let path: PathBuf = ["collection", "Khemmis", "Hunted"].iter().collect();
		let request = service.request_builder().browse(&path);
		let response = service.fetch_json::<_, Vec<index::CollectionFile>>(&request);
		assert_eq!(response.status(), StatusCode::OK);
		let entries = response.body();
		assert_eq!(entries.len(), 5);
	}
}

#[test]
fn test_service_flatten() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();
	service.index();

	{
		let request = service.request_builder().flatten(&PathBuf::new());
		let response = service.fetch_json::<_, Vec<index::Song>>(&request);
		assert_eq!(response.status(), StatusCode::OK);
		let entries = response.body();
		assert_eq!(entries.len(), 13);
	}

	{
		let request = service.request_builder().flatten(Path::new("collection"));
		let response = service.fetch_json::<_, Vec<index::Song>>(&request);
		assert_eq!(response.status(), StatusCode::OK);
		let entries = response.body();
		assert_eq!(entries.len(), 13);
	}
}

#[test]
fn test_service_random() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();
	service.index();

	let request = service.request_builder().random();
	let response = service.fetch_json::<_, Vec<index::Directory>>(&request);
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 3);
}

#[test]
fn test_service_recent() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();
	service.index();

	let request = service.request_builder().recent();
	let response = service.fetch_json::<_, Vec<index::Directory>>(&request);
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 3);
}

#[test]
fn test_service_search() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();
	service.index();

	{
		let request = service.request_builder().search("");
		let response = service.fetch_json::<_, Vec<index::CollectionFile>>(&request);
		assert_eq!(response.status(), StatusCode::OK);
	}

	{
		let request = service.request_builder().search("door");
		let response = service.fetch_json::<_, Vec<index::CollectionFile>>(&request);
		let results = response.body();
		assert_eq!(results.len(), 1);
		match results[0] {
			index::CollectionFile::Song(ref s) => {
				assert_eq!(s.title, Some("Beyond The Door".into()))
			}
			_ => panic!(),
		}
	}
}
#[test]
fn test_service_audio() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();
	service.index();

	let path: PathBuf = ["collection", "Khemmis", "Hunted", "02 - Candlelight.mp3"]
		.iter()
		.collect();

	{
		let request = service.request_builder().audio(&path);
		let response = service.fetch_bytes(&request);
		assert_eq!(response.status(), StatusCode::OK);
		assert_eq!(response.body().len(), 24_142);
	}

	{
		let mut request = service.request_builder().audio(&path);
		let headers = request.headers_mut();
		headers.append(RANGE, HeaderValue::from_str("bytes=100-299").unwrap());
		let response = service.fetch_bytes(&request);
		assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
		assert_eq!(response.body().len(), 200);
		assert_eq!(response.headers().get(CONTENT_LENGTH).unwrap(), "200");
	}
}

#[test]
fn test_service_thumbnail() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();
	service.index();

	let path: PathBuf = ["collection", "Khemmis", "Hunted", "Folder.jpg"]
		.iter()
		.collect();

	let pad = None;
	let request = service.request_builder().thumbnail(&path, pad);
	let response = service.fetch_bytes(&request);
	assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn test_service_playlists() {
	let mut service = ServiceType::new(&format!("{}{}", TEST_DB_PREFIX, line!()));
	service.complete_initial_setup();
	service.login();
	service.index();

	let list_playlists = service.request_builder().playlists();

	// List some songs
	let playlist_name = "my_playlist";
	let my_songs = {
		let request = service.request_builder().flatten(&PathBuf::new());
		let response = service.fetch_json::<_, Vec<index::Song>>(&request);
		assert_eq!(response.status(), StatusCode::OK);
		let mut my_songs = response.into_body();
		my_songs.pop();
		my_songs.pop();
		my_songs
	};

	// Verify no existing playlists
	{
		let response = service.fetch_json::<_, Vec<dto::ListPlaylistsEntry>>(&list_playlists);
		assert_eq!(response.status(), StatusCode::OK);
		let playlists = response.body();
		assert_eq!(playlists.len(), 0);
	}

	// Store a playlist
	{
		let my_playlist = dto::SavePlaylistInput {
			tracks: my_songs.iter().map(|s| s.path.clone()).collect(),
		};
		let request = service
			.request_builder()
			.save_playlist(playlist_name, my_playlist);
		let response = service.process(&request);
		assert_eq!(response.status(), StatusCode::OK);
	}

	// Verify new playlist is listed
	{
		let response = service.fetch_json::<_, Vec<dto::ListPlaylistsEntry>>(&list_playlists);
		assert_eq!(response.status(), StatusCode::OK);
		let playlists = response.body();
		assert_eq!(
			playlists,
			&vec![dto::ListPlaylistsEntry {
				name: playlist_name.to_owned()
			}]
		);
	}

	// Verify content of new playlist
	{
		let request = service.request_builder().read_playlist(playlist_name);
		let response = service.fetch_json::<_, Vec<index::Song>>(&request);
		assert_eq!(response.status(), StatusCode::OK);
		let songs = response.body();
		assert_eq!(songs, &my_songs);
	}

	// Delete playlist
	{
		let request = service.request_builder().delete_playlist(playlist_name);
		let response = service.process(&request);
		assert_eq!(response.status(), StatusCode::OK);
	}

	// Verify updated listing
	{
		let response = service.fetch_json::<_, Vec<dto::ListPlaylistsEntry>>(&list_playlists);
		let playlists = response.body();
		assert_eq!(playlists.len(), 0);
	}
}
