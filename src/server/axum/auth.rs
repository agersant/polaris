use axum::extract::{FromRef, FromRequestParts, Query};
use headers::authorization::{Bearer, Credentials};
use http::request::Parts;

use crate::{
	app::{auth, config},
	server::{dto, error::APIError},
};

#[derive(Debug)]
pub struct Auth {
	username: String,
}

impl Auth {
	pub fn get_username(&self) -> &String {
		&self.username
	}
}

impl<S> FromRequestParts<S> for Auth
where
	config::Manager: FromRef<S>,
	S: Send + Sync,
{
	type Rejection = APIError;

	async fn from_request_parts(parts: &mut Parts, app: &S) -> Result<Self, Self::Rejection> {
		let config_manager = config::Manager::from_ref(app);

		let header_token = parts
			.headers
			.get(http::header::AUTHORIZATION)
			.and_then(Bearer::decode)
			.map(|b| b.token().to_string());

		let query_token = Query::<dto::AuthQueryParameters>::try_from_uri(&parts.uri)
			.ok()
			.map(|p| p.auth_token.to_string());

		let Some(token) = query_token.or(header_token) else {
			return Err(APIError::AuthenticationRequired);
		};

		let authorization = config_manager
			.authenticate(&auth::Token(token), auth::Scope::PolarisAuth)
			.await?;

		Ok(Auth {
			username: authorization.username,
		})
	}
}

#[derive(Debug)]
pub struct AdminRights {
	auth: Option<Auth>,
}

impl AdminRights {
	pub fn get_auth(&self) -> &Option<Auth> {
		&self.auth
	}
}

impl<S> FromRequestParts<S> for AdminRights
where
	config::Manager: FromRef<S>,
	S: Send + Sync,
{
	type Rejection = APIError;

	async fn from_request_parts(parts: &mut Parts, app: &S) -> Result<Self, Self::Rejection> {
		let config_manager = config::Manager::from_ref(app);

		let user_count = config_manager.get_users().await.len();
		if user_count == 0 {
			return Ok(AdminRights { auth: None });
		}

		let auth = Auth::from_request_parts(parts, app).await?;
		if config_manager.get_user(&auth.username).await?.is_admin() {
			Ok(AdminRights { auth: Some(auth) })
		} else {
			Err(APIError::AdminPermissionRequired)
		}
	}
}
