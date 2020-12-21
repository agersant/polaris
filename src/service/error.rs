use thiserror::Error;

use crate::app::index::QueryError;
use crate::app::{config, playlist, settings, user};

#[derive(Error, Debug)]
pub enum APIError {
	#[error("Incorrect Credentials")]
	IncorrectCredentials,
	#[error("EmptyUsername")]
	EmptyUsername,
	#[error("EmptyPassword")]
	EmptyPassword,
	#[error("Cannot delete your own account")]
	DeletingOwnAccount,
	#[error("Cannot remove your own admin privilege")]
	OwnAdminPrivilegeRemoval,
	#[error("Audio file could not be opened")]
	AudioFileIOError,
	#[error("Thumbnail file could not be opened")]
	ThumbnailFileIOError,
	#[error("No last.fm account has been linked")]
	LastFMAccountNotLinked,
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

impl From<config::Error> for APIError {
	fn from(error: config::Error) -> APIError {
		match error {
			config::Error::Unspecified => APIError::Unspecified,
		}
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

impl From<settings::Error> for APIError {
	fn from(error: settings::Error) -> APIError {
		match error {
			settings::Error::AuthSecretNotFound => APIError::Unspecified,
			settings::Error::InvalidAuthSecret => APIError::Unspecified,
			settings::Error::IndexSleepDurationNotFound => APIError::Unspecified,
			settings::Error::IndexAlbumArtPatternNotFound => APIError::Unspecified,
			settings::Error::IndexAlbumArtPatternInvalid => APIError::Unspecified,
			settings::Error::Unspecified => APIError::Unspecified,
		}
	}
}

impl From<user::Error> for APIError {
	fn from(error: user::Error) -> APIError {
		match error {
			user::Error::EmptyUsername => APIError::EmptyUsername,
			user::Error::EmptyPassword => APIError::EmptyPassword,
			user::Error::IncorrectUsername => APIError::IncorrectCredentials,
			user::Error::IncorrectPassword => APIError::IncorrectCredentials,
			user::Error::InvalidAuthToken => APIError::IncorrectCredentials,
			user::Error::IncorrectAuthorizationScope => APIError::IncorrectCredentials,
			user::Error::Unspecified => APIError::Unspecified,
		}
	}
}
