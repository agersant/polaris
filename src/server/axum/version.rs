use axum::extract::FromRequestParts;
use http::request::Parts;

use crate::server::{error::APIError, APIMajorVersion};

impl<S> FromRequestParts<S> for APIMajorVersion
where
	S: Send + Sync,
{
	type Rejection = APIError;

	async fn from_request_parts(parts: &mut Parts, _app: &S) -> Result<Self, Self::Rejection> {
		let version_header = match parts.headers.get("Accept-Version").map(|h| h.to_str()) {
			Some(Ok(h)) => h,
			Some(Err(_)) => return Err(APIError::InvalidAPIVersionHeader),
			None => return Err(APIError::MissingAPIVersionHeader),
		};

		let version = match str::parse::<i32>(version_header) {
			Ok(v) => v,
			Err(_) => return Err(APIError::APIVersionHeaderParse),
		};

		APIMajorVersion::try_from(version)
	}
}
