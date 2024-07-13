use axum::{
	async_trait,
	extract::{FromRef, FromRequestParts, Query},
};
use headers::authorization::{Bearer, Credentials};
use http::request::Parts;

use crate::{
	app::user,
	server::{dto, error::APIError},
};

#[derive(Debug)]
pub struct Auth {
	username: String,
}

impl Auth {
	pub fn get_username(&self) -> &String {
		return &self.username;
	}
}

#[async_trait]
impl<S> FromRequestParts<S> for Auth
where
	user::Manager: FromRef<S>,
	S: Send + Sync,
{
	type Rejection = APIError;

	async fn from_request_parts(parts: &mut Parts, app: &S) -> Result<Self, Self::Rejection> {
		let user_manager = user::Manager::from_ref(app);

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

		let authorization = user_manager
			.authenticate(
				&user::AuthToken(token),
				user::AuthorizationScope::PolarisAuth,
			)
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
		return &self.auth;
	}
}

#[async_trait]
impl<S> FromRequestParts<S> for AdminRights
where
	user::Manager: FromRef<S>,
	S: Send + Sync,
{
	type Rejection = APIError;

	async fn from_request_parts(parts: &mut Parts, app: &S) -> Result<Self, Self::Rejection> {
		let user_manager = user::Manager::from_ref(app);

		let user_count = user_manager.count().await?;
		if user_count == 0 {
			return Ok(AdminRights { auth: None });
		}

		let auth = Auth::from_request_parts(parts, app).await?;
		if user_manager.is_admin(&auth.username).await? {
			Ok(AdminRights { auth: Some(auth) })
		} else {
			Err(APIError::AdminPermissionRequired)
		}
	}
}
