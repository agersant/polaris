use rocket::http::Status;
use rocket::local::Client;
use std::path::PathBuf;
use std::ops::Deref;
use std::sync::Arc;
use std::fs;

use crate::api;
use crate::db;
use crate::index;
use crate::server;

struct TestEnvironment {
	pub client: Client,
	command_sender: Arc<index::CommandSender>,
}

impl Drop for TestEnvironment {
	fn drop(&mut self) {
		self.command_sender.deref().exit().unwrap();
	}
}

fn get_test_environment(db_name: &str) -> TestEnvironment
{
	let mut db_path = PathBuf::new();
	db_path.push("test");
	db_path.push(db_name);
	if db_path.exists() {
		fs::remove_file(&db_path).unwrap();
	}

	let db = Arc::new(db::DB::new(&db_path).unwrap());

	let web_dir_path = PathBuf::from("web");
	let command_sender = index::init(db.clone());

	let server = server::get_server(5050, "/", "/api", &web_dir_path, db, command_sender.clone()).unwrap();
	let client = Client::new(server).unwrap();
	TestEnvironment { client, command_sender }
}

fn complete_initial_setup(client: &Client) {
	client.get("/api/initial_setup").dispatch();
	client.put("/api/settings")
	.body(r#"
	{	"users": [{ "name": "test_user", "password": "test_password", "admin": true }]
	,	"mount_dirs": [{ "name": "collection", "source": "test/collection" }]
	}"#)
	.dispatch();
}

#[test]
fn version() {
	let env = get_test_environment("api_version.sqlite");
	let client = &env.client;
	let mut response = client.get("/api/version").dispatch();

	assert_eq!(response.status(), Status::Ok);

	let response_body = response.body_string().unwrap();
	let response_json: api::Version = serde_json::from_str(&response_body).unwrap();
	assert_eq!(response_json, api::Version{major: 3, minor: 0});
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
		assert_eq!(response_json, api::InitialSetup{has_any_users: false});
	}

	complete_initial_setup(client);

	{
		let mut response = client.get("/api/initial_setup").dispatch();
		assert_eq!(response.status(), Status::Ok);
		let response_body = response.body_string().unwrap();
		let response_json: api::InitialSetup = serde_json::from_str(&response_body).unwrap();
		assert_eq!(response_json, api::InitialSetup{has_any_users: true});
	}
}

#[test]
fn settings() {
	// TODO
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
