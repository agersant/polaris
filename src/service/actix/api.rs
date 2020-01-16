use actix_http::ResponseBuilder;
use actix_web::{error, get, http::header, http::StatusCode, put, web, HttpResponse};
use anyhow::*;

use crate::config::{self, Config};
use crate::db::DB;
use crate::service::constants::*;
use crate::service::dto;
use crate::service::error::APIError;
use crate::user;

impl error::ResponseError for APIError {
	fn error_response(&self) -> HttpResponse {
		ResponseBuilder::new(self.status_code())
			.set_header(header::CONTENT_TYPE, "text/html; charset=utf-8")
			.body(self.to_string())
	}
	fn status_code(&self) -> StatusCode {
		match *self {
			APIError::IncorrectCredentials => StatusCode::UNAUTHORIZED,
			APIError::Unspecified => StatusCode::INTERNAL_SERVER_ERROR,
		}
	}
}

#[get("/version")]
pub async fn get_version() -> Result<HttpResponse, APIError> {
	let current_version = dto::Version {
		major: CURRENT_MAJOR_VERSION,
		minor: CURRENT_MINOR_VERSION,
	};
	Ok(HttpResponse::Ok().json(current_version))
}

#[get("/initial_setup")]
pub async fn get_initial_setup(db: web::Data<DB>) -> Result<HttpResponse, APIError> {
	let user_count = web::block(move || user::count(&db))
		.await
		.map_err(|_| anyhow!("Could not count users"))?;
	let initial_setup = dto::InitialSetup {
		has_any_users: user_count > 0,
	};
	Ok(HttpResponse::Ok().json(initial_setup))
}

#[put("/settings")]
pub async fn put_settings(
	db: web::Data<DB>,
	// _admin_rights: AdminRights, // TODO.important
	config: web::Json<Config>,
) -> Result<HttpResponse, APIError> {
	web::block(move || config::amend(&db, &config))
		.await
		.map_err(|_| anyhow!("Could not amend config"))?;
	Ok(HttpResponse::Ok().finish())
}
