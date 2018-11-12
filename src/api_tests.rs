use rocket::http::Status;
use rocket::local::Client;
use std::fs;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;

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

	let server =
		server::get_server(5050, "/", "/api", &web_dir_path, db, command_sender.clone()).unwrap();
	let client = Client::new(server).unwrap();
	TestEnvironment {
		client,
		command_sender,
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
	// TODO
}

#[test]
fn auth() {
	// TODO
}

#[test]
fn browse() {
	// TODO
}

#[test]
fn flatten() {
	// TODO
}

#[test]
fn random() {
	// TODO
}

#[test]
fn recent() {
	// TODO
}

#[test]
fn search() {
	// TODO
}

#[test]
fn serve() {
	// TODO
}

#[test]
fn playlists() {
	// TODO
}

#[test]
fn last_fm() {
	// TODO
}
