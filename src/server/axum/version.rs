use axum::{async_trait, extract::FromRequestParts};
use http::request::Parts;

use crate::server::{dto, error::APIError};

pub enum Version {
	V7,
	V8,
}

#[async_trait]
impl<S> FromRequestParts<S> for Version
where
	S: Send + Sync,
{
	type Rejection = APIError;

	async fn from_request_parts(parts: &mut Parts, _app: &S) -> Result<Self, Self::Rejection> {
		let version_header = match parts.headers.get("Accept-Version").map(|h| h.to_str()) {
			Some(Ok(h)) => h,
			Some(Err(_)) => return Err(APIError::InvalidAPIVersionHeader),
			None => return Ok(Version::V7), // TODO Drop support for implicit version in future release
		};

		let version: dto::Version = match serde_json::from_str(version_header) {
			Ok(v) => v,
			Err(_) => return Err(APIError::APIVersionHeaderParseError),
		};

		Ok(match version.major {
			7 => Version::V7,
			8 => Version::V8,
			_ => return Err(APIError::UnsupportedAPIVersion),
		})
	}
}
