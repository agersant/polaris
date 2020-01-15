use actix_web::{get, HttpResponse};

use crate::service::constants::*;
use crate::service::dto;

#[get("/version")]
pub async fn get_version() -> Result<HttpResponse, actix_web::Error> {
	let current_version = dto::Version {
		major: CURRENT_MAJOR_VERSION,
		minor: CURRENT_MINOR_VERSION,
	};
	Ok(HttpResponse::Ok().json(current_version))
}
