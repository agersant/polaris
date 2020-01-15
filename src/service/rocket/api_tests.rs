use rocket::http::hyper::header::*;
use rocket::http::uri::Uri;
use rocket::http::Status;
use rocket::local::Client;
use std::{thread, time};

use super::api;
use crate::config;
use crate::ddns;
use crate::index;
use crate::service::dto;
use crate::vfs;

use super::test::get_test_environment;

const TEST_USERNAME: &str = "test_user";
const TEST_PASSWORD: &str = "test_password";
const TEST_MOUNT_NAME: &str = "collection";
const TEST_MOUNT_SOURCE: &str = "test/collection";

fn complete_initial_setup(client: &Client) {
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
	let body = serde_json::to_string(&configuration).unwrap();
	let response = client.put("/api/settings").body(&body).dispatch();
	assert_eq!(response.status(), Status::Ok);
}

fn do_auth(client: &Client) {
	let credentials = api::AuthCredentials {
		username: TEST_USERNAME.into(),
		password: TEST_PASSWORD.into(),
	};
	let body = serde_json::to_string(&credentials).unwrap();
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
	let response_json: dto::Version = serde_json::from_str(&response_body).unwrap();
	assert_eq!(response_json, dto::Version { major: 4, minor: 0 });
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

	let body = serde_json::to_string(&configuration).unwrap();

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

	client.put("/api/settings").body(body).dispatch();

	{
		let mut response = client.get("/api/settings").dispatch();
		assert_eq!(response.status(), Status::Ok);
		let response_body = response.body_string().unwrap();
		let response_json: config::Config = serde_json::from_str(&response_body).unwrap();
		assert_eq!(response_json, configuration);
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
		let credentials = api::AuthCredentials {
			username: "garbage".into(),
			password: "garbage".into(),
		};
		let response = client
			.post("/api/auth")
			.body(serde_json::to_string(&credentials).unwrap())
			.dispatch();
		assert_eq!(response.status(), Status::Unauthorized);
	}
	{
		let credentials = api::AuthCredentials {
			username: TEST_USERNAME.into(),
			password: "garbage".into(),
		};
		let response = client
			.post("/api/auth")
			.body(serde_json::to_string(&credentials).unwrap())
			.dispatch();
		assert_eq!(response.status(), Status::Unauthorized);
	}
	{
		let credentials = api::AuthCredentials {
			username: TEST_USERNAME.into(),
			password: TEST_PASSWORD.into(),
		};
		let response = client
			.post("/api/auth")
			.body(serde_json::to_string(&credentials).unwrap())
			.dispatch();
		assert_eq!(response.status(), Status::Ok);
		assert!(response
			.cookies()
			.iter()
			.any(|cookie| cookie.name() == "username"));
		assert!(response
			.cookies()
			.iter()
			.any(|cookie| cookie.name() == "admin"));
		assert!(response
			.cookies()
			.iter()
			.any(|cookie| cookie.name() == "session"));
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
	let env = get_test_environment("api_serve.sqlite");
	let client = &env.client;
	complete_initial_setup(client);
	do_auth(client);
	env.update_index();

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
