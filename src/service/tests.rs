use function_name::named;

mod api;
mod static_files;

use crate::config;
use crate::service::dto;
use crate::vfs;

#[cfg(feature = "service-actix")]
pub use crate::service::actix::test::*;

#[cfg(feature = "service-rocket")]
pub use crate::service::rocket::test::*;

const TEST_USERNAME: &str = "test_user";
const TEST_PASSWORD: &str = "test_password";
const TEST_MOUNT_NAME: &str = "collection";
const TEST_MOUNT_SOURCE: &str = "test/collection";

#[named]
#[actix_rt::test]
async fn test_index() {
	let mut service = make_service(function_name!()).await;
	get(&mut service, "/").await;
}

#[named]
#[actix_rt::test]
async fn test_swagger_index() {
	let mut service = make_service(function_name!()).await;
	get(&mut service, "/swagger").await;
}

#[named]
#[actix_rt::test]
async fn test_swagger_index_with_trailing_slash() {
	let mut service = make_service(function_name!()).await;
	get(&mut service, "/swagger/").await;
}

async fn complete_initial_setup(service: &mut ServiceType) {
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
	put_json(service, "/api/settings", &configuration).await;
}

#[named]
#[actix_rt::test]
async fn test_version() {
	let mut service = make_service(function_name!()).await;
	let version: dto::Version = get_json(&mut service, "/api/version").await;
	assert_eq!(version, dto::Version { major: 4, minor: 0 });
}

#[named]
#[actix_rt::test]
async fn test_initial_setup() {
	let mut service = make_service(function_name!()).await;

	{
		let initial_setup: dto::InitialSetup = get_json(&mut service, "/api/initial_setup").await;
		assert_eq!(
			initial_setup,
			dto::InitialSetup {
				has_any_users: false
			}
		);
	}

	complete_initial_setup(&mut service).await;

	{
		let initial_setup: dto::InitialSetup = get_json(&mut service, "/api/initial_setup").await;
		assert_eq!(
			initial_setup,
			dto::InitialSetup {
				has_any_users: true
			}
		);
	}
}
