use http::StatusCode;

use crate::server::dto;
use crate::server::test::{protocol, ServiceType, TestService};
use crate::test_name;

#[tokio::test]
async fn get_ddns_config_requires_admin() {
	let mut service = ServiceType::new(&test_name!()).await;
	let request = protocol::get_ddns_config();
	service.complete_initial_setup().await;

	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

	service.login().await;
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn get_ddns_config_golden_path() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;

	let request = protocol::get_ddns_config();
	let response = service.fetch_json::<_, dto::DDNSConfig>(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn put_ddns_config_requires_admin() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;

	let request = protocol::put_ddns_config(dto::DDNSConfig {
		host: "host".to_owned(),
		username: "ddns_user".to_owned(),
		password: "ddns_password".to_owned(),
	});

	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

	service.login().await;
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn put_ddns_config_golden_path() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;

	let request = protocol::put_ddns_config(dto::DDNSConfig {
		host: "test".to_owned(),
		username: "test".to_owned(),
		password: "test".to_owned(),
	});
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
}
