use error::APIError;

mod dto;
mod error;

#[cfg(test)]
mod test;

pub enum APIMajorVersion {
	V7,
	V8,
}

impl TryFrom<i32> for APIMajorVersion {
	type Error = APIError;

	fn try_from(value: i32) -> Result<Self, Self::Error> {
		match value {
			7 => Ok(Self::V7),
			8 => Ok(Self::V8),
			_ => Err(APIError::UnsupportedAPIVersion),
		}
	}
}

pub const API_MAJOR_VERSION: i32 = 8;
pub const API_MINOR_VERSION: i32 = 0;

mod axum;
pub use axum::*;
