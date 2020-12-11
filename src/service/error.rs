use thiserror::Error;

use crate::app::index::QueryError;
use crate::app::playlist;

#[derive(Error, Debug)]
pub enum APIError {
	#[error("Incorrect Credentials")]
	IncorrectCredentials,
	#[error("Cannot remove own admin privilege")]
	OwnAdminPrivilegeRemoval,
	#[error("Audio file could not be opened")]
	AudioFileIOError,
	#[error("Thumbnail file could not be opened")]
	ThumbnailFileIOError,
	#[error("Could not decode content as base64 after linking last.fm account")]
	LastFMLinkContentBase64DecodeError,
	#[error("Could not decode content as UTF-8 after linking last.fm account")]
	LastFMLinkContentEncodingError,
	#[error("Path not found in virtual filesystem")]
	VFSPathNotFound,
	#[error("User not found")]
	UserNotFound,
	#[error("Playlist not found")]
	PlaylistNotFound,
	#[error("Unspecified")]
	Unspecified,
}

impl From<anyhow::Error> for APIError {
	fn from(_: anyhow::Error) -> Self {
		APIError::Unspecified
	}
}

impl From<playlist::Error> for APIError {
	fn from(error: playlist::Error) -> APIError {
		match error {
			playlist::Error::PlaylistNotFound => APIError::PlaylistNotFound,
			playlist::Error::UserNotFound => APIError::UserNotFound,
			playlist::Error::Unspecified => APIError::Unspecified,
		}
	}
}

impl From<QueryError> for APIError {
	fn from(error: QueryError) -> APIError {
		match error {
			QueryError::VFSPathNotFound => APIError::VFSPathNotFound,
			QueryError::Unspecified => APIError::Unspecified,
		}
	}
}
