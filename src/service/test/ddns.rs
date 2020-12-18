use http::StatusCode;

use crate::service::dto;
use crate::service::test::{protocol, ServiceType, TestService};
use crate::test_name;

#[test]
fn test_get_ddns_config_requires_admin() {
	let mut service = ServiceType::new(&test_name!());
	let request = protocol::get_ddns_config();
	service.complete_initial_setup();

	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

	service.login();
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[test]
fn test_get_ddns_config_golden_path() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login_admin();

	let request = protocol::get_ddns_config();
	let response = service.fetch_json::<_, dto::DDNSConfig>(&request);
	assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn test_put_ddns_config_requires_admin() {
	let mut service = ServiceType::new(&test_name!());
	let request = protocol::put_ddns_config(dto::DDNSConfig {
		host: "test".to_owned(),
		username: "test".to_owned(),
		password: "test".to_owned(),
	});
	service.complete_initial_setup();

	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

	service.login();
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[test]
fn test_put_ddns_config_golden_path() {
	let mut service = ServiceType::new(&test_name!());
	service.complete_initial_setup();
	service.login_admin();

	let request = protocol::put_ddns_config(dto::DDNSConfig {
		host: "test".to_owned(),
		username: "test".to_owned(),
		password: "test".to_owned(),
	});
	let response = service.fetch(&request);
	assert_eq!(response.status(), StatusCode::OK);
}
