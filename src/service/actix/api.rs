use actix_web::{
	get, http::StatusCode, put, web::Data, web::Json, web::ServiceConfig, ResponseError,
};

use crate::config::{self, Config};
use crate::db::DB;
use crate::service::{constants::*, dto, error::*};
use crate::user;

pub fn make_config() -> impl FnOnce(&mut ServiceConfig) + Clone {
	move |cfg: &mut ServiceConfig| {
		cfg.service(version);
		cfg.service(initial_setup);
		cfg.service(put_settings);
	}
}

impl ResponseError for APIError {
	fn status_code(&self) -> StatusCode {
		match self {
			APIError::IncorrectCredentials => StatusCode::UNAUTHORIZED,
			APIError::OwnAdminPrivilegeRemoval => StatusCode::CONFLICT,
			APIError::Unspecified => StatusCode::INTERNAL_SERVER_ERROR,
		}
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

#[get("/initial_setup")]
async fn initial_setup(db: Data<DB>) -> Result<Json<dto::InitialSetup>, APIError> {
	let initial_setup = dto::InitialSetup {
		has_any_users: user::count(&db)? > 0,
	};
	Ok(Json(initial_setup))
}

#[put("/settings")]
async fn put_settings(db: Data<DB>, config: Json<Config>) -> Result<&'static str, APIError> {
	// TODO config should be a dto type

	// TODO permissions

	// Do not let users remove their own admin rights
	// TODO
	// if let Some(auth) = &admin_rights.auth {
	// 	if let Some(users) = &config.users {
	// 		for user in users {
	// 			if auth.username == user.name && !user.admin {
	// 				return Err(APIError::OwnAdminPrivilegeRemoval);
	// 			}
	// 		}
	// 	}
	// }
	config::amend(&db, &config)?;
	Ok("")
}
