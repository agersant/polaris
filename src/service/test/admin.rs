use http::StatusCode;

use crate::app::index;
use crate::service::dto;
use crate::service::test::{protocol, ServiceType, TestService};
use crate::test_name;

#[actix_web::test]
async fn returns_api_version() {
	let mut service = ServiceType::new(&test_name!()).await;
	let request = protocol::version();
	let response = service.fetch_json::<_, dto::Version>(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
}

#[actix_web::test]
async fn initial_setup_golden_path() {
	let mut service = ServiceType::new(&test_name!()).await;
	let request = protocol::initial_setup();
	{
		let response = service.fetch_json::<_, dto::InitialSetup>(&request).await;
		assert_eq!(response.status(), StatusCode::OK);
		let initial_setup = response.body();
		assert_eq!(
			initial_setup,
			&dto::InitialSetup {
				has_any_users: false
			}
		);
	}
	service.complete_initial_setup().await;
	{
		let response = service.fetch_json::<_, dto::InitialSetup>(&request).await;
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

#[actix_web::test]
async fn trigger_index_golden_path() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;

	let request = protocol::random();

	let response = service
		.fetch_json::<_, Vec<index::Directory>>(&request)
		.await;
	let entries = response.body();
	assert_eq!(entries.len(), 0);

	service.index().await;

	let response = service
		.fetch_json::<_, Vec<index::Directory>>(&request)
		.await;
	let entries = response.body();
	assert_eq!(entries.len(), 3);
}

#[actix_web::test]
async fn trigger_index_requires_auth() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	let request = protocol::trigger_index();
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn trigger_index_requires_admin() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login().await;
	let request = protocol::trigger_index();
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
