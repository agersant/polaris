use thiserror::Error;

use crate::app::index::QueryError;
use crate::app::{config, ddns, playlist, settings, user, vfs};
use crate::db;

#[derive(Error, Debug)]
pub enum APIError {
	#[error("Authentication is required")]
	AuthenticationRequired,
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
	#[error("Internal server error")]
	Internal,
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
			config::Error::Ddns(e) => e.into(),
			config::Error::Settings(e) => e.into(),
			config::Error::User(e) => e.into(),
			config::Error::Vfs(e) => e.into(),
		}
	}
}

impl From<playlist::Error> for APIError {
	fn from(error: playlist::Error) -> APIError {
		match error {
			playlist::Error::DatabaseConnection(e) => e.into(),
			playlist::Error::PlaylistNotFound => APIError::PlaylistNotFound,
			playlist::Error::UserNotFound => APIError::UserNotFound,
			playlist::Error::Vfs(e) => e.into(),
			playlist::Error::Unspecified => APIError::Unspecified,
		}
	}
}

impl From<QueryError> for APIError {
	fn from(error: QueryError) -> APIError {
		match error {
			QueryError::DatabaseConnection(e) => e.into(),
			QueryError::Vfs(e) => e.into(),
			QueryError::Unspecified => APIError::Unspecified,
		}
	}
}

impl From<settings::Error> for APIError {
	fn from(error: settings::Error) -> APIError {
		match error {
			settings::Error::AuthSecretNotFound => APIError::Internal,
			settings::Error::DatabaseConnection(e) => e.into(),
			settings::Error::InvalidAuthSecret => APIError::Internal,
			settings::Error::MiscSettingsNotFound => APIError::Internal,
			settings::Error::IndexAlbumArtPatternInvalid => APIError::Internal,
			settings::Error::Database(_) => APIError::Internal,
			settings::Error::Unspecified => APIError::Unspecified,
		}
	}
}

impl From<user::Error> for APIError {
	fn from(error: user::Error) -> APIError {
		match error {
			user::Error::DatabaseConnection(e) => e.into(),
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

impl From<vfs::Error> for APIError {
	fn from(error: vfs::Error) -> APIError {
		match error {
			vfs::Error::CouldNotMapToVirtualPath(_) => APIError::VFSPathNotFound,
			vfs::Error::CouldNotMapToRealPath(_) => APIError::VFSPathNotFound,
			vfs::Error::Database(_) => APIError::Internal,
			vfs::Error::DatabaseConnection(e) => e.into(),
			vfs::Error::Unspecified => APIError::Unspecified,
		}
	}
}

impl From<ddns::Error> for APIError {
	fn from(error: ddns::Error) -> APIError {
		match error {
			ddns::Error::DatabaseConnection(e) => e.into(),
			ddns::Error::UpdateQueryFailed(_) => APIError::Internal,
			ddns::Error::Database(_) => APIError::Internal,
			ddns::Error::Unspecified => APIError::Unspecified,
		}
	}
}

impl From<db::Error> for APIError {
	fn from(error: db::Error) -> APIError {
		match error {
			db::Error::ConnectionPoolBuild => APIError::Internal,
			db::Error::ConnectionPool => APIError::Internal,
			db::Error::Io(_, _) => APIError::Internal,
			db::Error::Migration => APIError::Internal,
		}
	}
}
