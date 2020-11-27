use actix_web::{get, web::Json, web::ServiceConfig};

use crate::service::constants::*;
use crate::service::dto;

pub fn make_config() -> impl FnOnce(&mut ServiceConfig) + Clone {
	move |cfg: &mut ServiceConfig| {
		cfg.service(version);
	}
}

#[get("/version")]
async fn version() -> Json<dto::Version> {
	let current_version = dto::Version {
		major: API_MAJOR_VERSION,
		minor: API_MINOR_VERSION,
	};
	Json(current_version)
}
