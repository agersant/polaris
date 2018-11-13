use rocket::http::uri::Uri;
use rocket::http::Status;
use rocket::local::Client;
use std::fs;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;
use std::{thread, time};

use crate::api;
use crate::config;
use crate::db;
use crate::ddns;
use crate::index;
use crate::server;
use crate::vfs;

const TEST_USERNAME: &str = "test_user";
const TEST_PASSWORD: &str = "test_password";
const TEST_MOUNT_NAME: &str = "collection";
const TEST_MOUNT_SOURCE: &str = "test/collection";

struct TestEnvironment {
	pub client: Client,
	command_sender: Arc<index::CommandSender>,
	db: Arc<db::DB>,
}

impl TestEnvironment {
	pub fn update_index(&self) {
		index::update(self.db.deref()).unwrap();
	}
}

impl Drop for TestEnvironment {
	fn drop(&mut self) {
		self.command_sender.deref().exit().unwrap();
	}
}

fn get_test_environment(db_name: &str) -> TestEnvironment {
	let mut db_path = PathBuf::new();
	db_path.push("test");
	db_path.push(db_name);
	if db_path.exists() {
		fs::remove_file(&db_path).unwrap();
	}

	let db = Arc::new(db::DB::new(&db_path).unwrap());

	let web_dir_path = PathBuf::from("web");
	let command_sender = index::init(db.clone());

	let server = server::get_server(
		5050,
		"/",
		"/api",
		&web_dir_path,
		db.clone(),
		command_sender.clone(),
	)
	.unwrap();
	let client = Client::new(server).unwrap();
	TestEnvironment {
		client,
		command_sender,
		db,
	}
}

fn complete_initial_setup(client: &Client) {
	let body = format!(
		r#"
	{{	"users": [{{ "name": "{}", "password": "{}", "admin": true }}]
	,	"mount_dirs": [{{ "name": "{}", "source": "{}" }}]
	}}"#,
		TEST_USERNAME, TEST_PASSWORD, TEST_MOUNT_NAME, TEST_MOUNT_SOURCE
	);

	let response = client.put("/api/settings").body(&body).dispatch();
	assert_eq!(response.status(), Status::Ok);
}

fn do_auth(client: &Client) {
	let body = format!(
		r#"
	{{	"username": "{}"
	,	"password": "{}"
	}}"#,
		TEST_USERNAME, TEST_PASSWORD
	);

	let response = client.post("/api/auth").body(body).dispatch();
	assert_eq!(response.status(), Status::Ok);
}

#[test]
fn version() {
	let env = get_test_environment("api_version.sqlite");
	let client = &env.client;
	let mut response = client.get("/api/version").dispatch();

	assert_eq!(response.status(), Status::Ok);

	let response_body = response.body_string().unwrap();
	let response_json: api::Version = serde_json::from_str(&response_body).unwrap();
	assert_eq!(response_json, api::Version { major: 3, minor: 0 });
}

#[test]
fn initial_setup() {
	let env = get_test_environment("api_initial_setup.sqlite");
	let client = &env.client;

	{
		let mut response = client.get("/api/initial_setup").dispatch();
		assert_eq!(response.status(), Status::Ok);
		let response_body = response.body_string().unwrap();
		let response_json: api::InitialSetup = serde_json::from_str(&response_body).unwrap();
		assert_eq!(
			response_json,
			api::InitialSetup {
				has_any_users: false
			}
		);
	}

	complete_initial_setup(client);

	{
		let mut response = client.get("/api/initial_setup").dispatch();
		assert_eq!(response.status(), Status::Ok);
		let response_body = response.body_string().unwrap();
		let response_json: api::InitialSetup = serde_json::from_str(&response_body).unwrap();
		assert_eq!(
			response_json,
			api::InitialSetup {
				has_any_users: true
			}
		);
	}
}

#[test]
fn settings() {
	let env = get_test_environment("api_settings.sqlite");
	let client = &env.client;
	complete_initial_setup(client);

	{
		let response = client.get("/api/settings").dispatch();
		assert_eq!(response.status(), Status::Unauthorized);
	}

	do_auth(client);

	{
		let mut response = client.get("/api/settings").dispatch();
		assert_eq!(response.status(), Status::Ok);
		let response_body = response.body_string().unwrap();
		let response_json: config::Config = serde_json::from_str(&response_body).unwrap();
		assert_eq!(
			response_json,
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

	client
		.put("/api/settings")
		.body(
			r#"
		{	"users":	[	{ "name": "test_user", "password": "test_password", "admin": true }
						,	{ "name": "other_user", "password": "other_password", "admin": false }
						]
		,	"mount_dirs":	[	{ "name": "collection", "source": "test/collection" }
							, 	{ "name": "more_music", "source": "test/collection" }
							]
		,	"album_art_pattern": "my_pattern"
		,	"reindex_every_n_seconds": 3600
		,	"prefix_url": "my_prefix"
		,	"ydns": { "host": "my_host", "username": "my_username", "password": "my_password" }
		}"#,
		)
		.dispatch();

	{
		let mut response = client.get("/api/settings").dispatch();
		assert_eq!(response.status(), Status::Ok);
		let response_body = response.body_string().unwrap();
		let response_json: config::Config = serde_json::from_str(&response_body).unwrap();

		assert_eq!(
			response_json,
			config::Config {
				album_art_pattern: Some("my_pattern".to_owned()),
				reindex_every_n_seconds: Some(3600),
				mount_dirs: Some(vec![
					vfs::MountPoint {
						name: TEST_MOUNT_NAME.into(),
						source: TEST_MOUNT_SOURCE.into()
					},
					vfs::MountPoint {
						name: "more_music".into(),
						source: "test/collection".into()
					}
				]),
				prefix_url: Some("my_prefix".to_owned()),
				users: Some(vec![
					config::ConfigUser {
						name: "test_user".into(),
						password: "".into(),
						admin: true
					},
					config::ConfigUser {
						name: "other_user".into(),
						password: "".into(),
						admin: false
					}
				]),
				ydns: Some(ddns::DDNSConfig {
					host: "my_host".into(),
					username: "my_username".into(),
					password: "my_password".into()
				}),
			}
		);
	}
}

#[test]
fn preferences() {
	// TODO
}

#[test]
fn trigger_index() {
	let env = get_test_environment("api_trigger_index.sqlite");
	let client = &env.client;
	complete_initial_setup(client);
	do_auth(client);

	{
		let mut response = client.get("/api/random").dispatch();
		assert_eq!(response.status(), Status::Ok);
		let response_body = response.body_string().unwrap();
		let response_json: Vec<index::Directory> = serde_json::from_str(&response_body).unwrap();
		assert_eq!(response_json.len(), 0);
	}

	{
		let response = client.post("/api/trigger_index").dispatch();
		assert_eq!(response.status(), Status::Ok);
	}

	let timeout = time::Duration::from_secs(5);
	thread::sleep(timeout);

	{
		let mut response = client.get("/api/random").dispatch();
		assert_eq!(response.status(), Status::Ok);
		let response_body = response.body_string().unwrap();
		let response_json: Vec<index::Directory> = serde_json::from_str(&response_body).unwrap();
		assert_eq!(response_json.len(), 2);
	}
}

#[test]
fn auth() {
	let env = get_test_environment("api_auth.sqlite");
	let client = &env.client;
	complete_initial_setup(client);

	{
		let response = client
			.post("/api/auth")
			.body(r#"{"username": "garbage", "password": "garbage"}"#)
			.dispatch();
		assert_eq!(response.status(), Status::Unauthorized);
	}
	{
		let response = client
			.post("/api/auth")
			.body(format!(
				r#"{{"username": "{}", "password": "garbage"}}"#,
				TEST_USERNAME
			))
			.dispatch();
		assert_eq!(response.status(), Status::Unauthorized);
	}
	{
		let response = client
			.post("/api/auth")
			.body(format!(
				r#"{{"username": "{}", "password": "{}"}}"#,
				TEST_USERNAME, TEST_PASSWORD
			))
			.dispatch();
		assert_eq!(response.status(), Status::Ok);
		assert_eq!(response.cookies()[0].name(), "session");
	}
}

#[test]
fn browse() {
	let env = get_test_environment("api_browse.sqlite");
	let client = &env.client;
	complete_initial_setup(client);
	do_auth(client);
	env.update_index();

	{
		let mut response = client.get("/api/browse").dispatch();
		assert_eq!(response.status(), Status::Ok);
		let response_body = response.body_string().unwrap();
		let response_json: Vec<index::CollectionFile> =
			serde_json::from_str(&response_body).unwrap();
		assert_eq!(response_json.len(), 1);
	}

	let mut next;
	{
		let mut response = client.get("/api/browse/collection").dispatch();
		assert_eq!(response.status(), Status::Ok);
		let response_body = response.body_string().unwrap();
		let response_json: Vec<index::CollectionFile> =
			serde_json::from_str(&response_body).unwrap();
		assert_eq!(response_json.len(), 2);

		match response_json[0] {
			index::CollectionFile::Directory(ref d) => {
				next = d.path.clone();
			}
			_ => panic!(),
		}
	}

	// /api/browse/collection/Khemmis
	{
		let url = format!("/api/browse/{}", Uri::percent_encode(&next));
		let mut response = client.get(url).dispatch();
		assert_eq!(response.status(), Status::Ok);
		let response_body = response.body_string().unwrap();
		let response_json: Vec<index::CollectionFile> =
			serde_json::from_str(&response_body).unwrap();
		assert_eq!(response_json.len(), 1);
		match response_json[0] {
			index::CollectionFile::Directory(ref d) => {
				next = d.path.clone();
			}
			_ => panic!(),
		}
	}

	// /api/browse/collection/Khemmis/Hunted
	{
		let url = format!("/api/browse/{}", Uri::percent_encode(&next));
		let mut response = client.get(url).dispatch();
		assert_eq!(response.status(), Status::Ok);
		let response_body = response.body_string().unwrap();
		let response_json: Vec<index::CollectionFile> =
			serde_json::from_str(&response_body).unwrap();
		assert_eq!(response_json.len(), 5);
	}
}

#[test]
fn flatten() {
	let env = get_test_environment("api_flatten.sqlite");
	let client = &env.client;
	complete_initial_setup(client);
	do_auth(client);
	env.update_index();

	{
		let mut response = client.get("/api/flatten").dispatch();
		assert_eq!(response.status(), Status::Ok);
		let response_body = response.body_string().unwrap();
		let response_json: Vec<index::Song> = serde_json::from_str(&response_body).unwrap();
		assert_eq!(response_json.len(), 12);
	}

	{
		let mut response = client.get("/api/flatten/collection").dispatch();
		assert_eq!(response.status(), Status::Ok);
		let response_body = response.body_string().unwrap();
		let response_json: Vec<index::Song> = serde_json::from_str(&response_body).unwrap();
		assert_eq!(response_json.len(), 12);
	}
}

#[test]
fn random() {
	let env = get_test_environment("api_random.sqlite");
	let client = &env.client;
	complete_initial_setup(client);
	do_auth(client);
	env.update_index();

	let mut response = client.get("/api/random").dispatch();
	assert_eq!(response.status(), Status::Ok);
	let response_body = response.body_string().unwrap();
	let response_json: Vec<index::Directory> = serde_json::from_str(&response_body).unwrap();
	assert_eq!(response_json.len(), 2);
}

#[test]
fn recent() {
	let env = get_test_environment("api_recent.sqlite");
	let client = &env.client;
	complete_initial_setup(client);
	do_auth(client);
	env.update_index();

	let mut response = client.get("/api/recent").dispatch();
	assert_eq!(response.status(), Status::Ok);
	let response_body = response.body_string().unwrap();
	let response_json: Vec<index::Directory> = serde_json::from_str(&response_body).unwrap();
	assert_eq!(response_json.len(), 2);
}

#[test]
fn search() {
	let env = get_test_environment("api_search.sqlite");
	let client = &env.client;
	complete_initial_setup(client);
	do_auth(client);
	env.update_index();

	let mut response = client.get("/api/search/door").dispatch();
	assert_eq!(response.status(), Status::Ok);
	let response_body = response.body_string().unwrap();
	let response_json: Vec<index::CollectionFile> = serde_json::from_str(&response_body).unwrap();
	assert_eq!(response_json.len(), 1);
	match response_json[0] {
		index::CollectionFile::Song(ref s) => assert_eq!(s.title, Some("Beyond The Door".into())),
		_ => panic!(),
	}
}

#[test]
fn serve() {
	// TODO
}

#[test]
fn playlists() {
	let env = get_test_environment("api_playlists.sqlite");
	let client = &env.client;
	complete_initial_setup(client);
	do_auth(client);
	env.update_index();

	{
		let mut response = client.get("/api/playlists").dispatch();
		assert_eq!(response.status(), Status::Ok);
		let response_body = response.body_string().unwrap();
		let response_json: Vec<api::ListPlaylistsEntry> =
			serde_json::from_str(&response_body).unwrap();
		assert_eq!(response_json.len(), 0);
	}

	{
		let songs: Vec<index::Song>;
		{
			let mut response = client.get("/api/flatten").dispatch();
			let response_body = response.body_string().unwrap();
			songs = serde_json::from_str(&response_body).unwrap();
		}
		let my_playlist = api::SavePlaylistInput {
			tracks: songs[2..6].into_iter().map(|s| s.path.clone()).collect(),
		};
		let response = client
			.put("/api/playlist/my_playlist")
			.body(serde_json::to_string(&my_playlist).unwrap())
			.dispatch();
		assert_eq!(response.status(), Status::Ok);
	}

	{
		let mut response = client.get("/api/playlists").dispatch();
		assert_eq!(response.status(), Status::Ok);
		let response_body = response.body_string().unwrap();
		let response_json: Vec<api::ListPlaylistsEntry> =
			serde_json::from_str(&response_body).unwrap();
		assert_eq!(
			response_json,
			vec![api::ListPlaylistsEntry {
				name: "my_playlist".into()
			}]
		);
	}

	{
		let mut response = client.get("/api/playlist/my_playlist").dispatch();
		assert_eq!(response.status(), Status::Ok);
		let response_body = response.body_string().unwrap();
		let response_json: Vec<index::Song> = serde_json::from_str(&response_body).unwrap();
		assert_eq!(response_json.len(), 4);
	}

	{
		let response = client.delete("/api/playlist/my_playlist").dispatch();
		assert_eq!(response.status(), Status::Ok);
	}

	{
		let mut response = client.get("/api/playlists").dispatch();
		assert_eq!(response.status(), Status::Ok);
		let response_body = response.body_string().unwrap();
		let response_json: Vec<api::ListPlaylistsEntry> =
			serde_json::from_str(&response_body).unwrap();
		assert_eq!(response_json.len(), 0);
	}
}

#[test]
fn last_fm() {
	// TODO
}
