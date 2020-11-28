use actix_web::error::{ErrorInternalServerError, ErrorUnauthorized};
use actix_web::{
	dev::Payload, get, http::StatusCode, put, web::Data, web::Json, web::ServiceConfig,
	FromRequest, HttpRequest, ResponseError,
};
use futures_util::future::{err, ok, Ready};
use std::future::Future;
use std::pin::Pin;

use crate::config::{self, Config};
use crate::db::DB;
use crate::service::{constants::*, dto, error::*};
use crate::user;

pub fn make_config() -> impl FnOnce(&mut ServiceConfig) + Clone {
	move |cfg: &mut ServiceConfig| {
		cfg.service(version)
			.service(initial_setup)
			.service(put_settings);
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

struct Auth {
	username: String,
}

impl FromRequest for Auth {
	type Error = actix_web::Error;
	type Future = Ready<Result<Self, Self::Error>>;
	type Config = ();

	fn from_request(request: &HttpRequest, _payload: &mut Payload) -> Self::Future {
		ok(Auth {
			username: "TODO".to_owned(),
		})
	}
}

struct AdminRights {
	auth: Option<Auth>,
}

impl FromRequest for AdminRights {
	type Error = actix_web::Error;
	type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;
	type Config = ();

	fn from_request(request: &HttpRequest, payload: &mut Payload) -> Self::Future {
		let db = match request.app_data::<DB>() {
			Some(db) => db.clone(),
			None => return Box::pin(err(ErrorInternalServerError(APIError::Unspecified))),
		};

		match user::count(&db) {
			Err(_) => return Box::pin(err(ErrorInternalServerError(APIError::Unspecified))),
			Ok(0) => return Box::pin(ok(AdminRights { auth: None })),
			_ => (),
		};

		let auth_future = Auth::from_request(request, payload);

		Box::pin(async move {
			let auth = auth_future.await?;
			match user::is_admin(&db, &auth.username) {
				Err(_) => Err(ErrorInternalServerError(APIError::Unspecified)),
				Ok(true) => Ok(AdminRights { auth: Some(auth) }),
				Ok(false) => Err(ErrorUnauthorized(APIError::Unspecified)),
			}
		})
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
async fn put_settings(
	admin_rights: AdminRights,
	db: Data<DB>,
	config: Json<Config>,
) -> Result<&'static str, APIError> {
	// TODO config should be a dto type

	// TODO permissions

	// Do not let users remove their own admin rights
	if let Some(auth) = &admin_rights.auth {
		if let Some(users) = &config.users {
			for user in users {
				if auth.username == user.name && !user.admin {
					return Err(APIError::OwnAdminPrivilegeRemoval);
				}
			}
		}
	}

	config::amend(&db, &config)?;
	Ok("")
}
