use thiserror::Error;

use crate::app::index::QueryError;
use crate::app::{config, ddns, lastfm, playlist, settings, thumbnail, user, vfs};
use crate::db;

#[derive(Error, Debug)]
pub enum APIError {
	#[error("Administrator permission is required")]
	AdminPermissionRequired,
	#[error("Authentication is required")]
	AuthenticationRequired,
	#[error("Incorrect Credentials")]
	IncorrectCredentials,
	#[error("EmbeddedArtworkNotFound")]
	EmbeddedArtworkNotFound,
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
	#[error("Song not found")]
	SongMetadataNotFound,
	#[error("Internal server error")]
	Internal,
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
			playlist::Error::Database(_) => APIError::Internal,
			playlist::Error::DatabaseConnection(e) => e.into(),
			playlist::Error::PlaylistNotFound => APIError::PlaylistNotFound,
			playlist::Error::UserNotFound => APIError::UserNotFound,
			playlist::Error::Vfs(e) => e.into(),
		}
	}
}

impl From<QueryError> for APIError {
	fn from(error: QueryError) -> APIError {
		match error {
			QueryError::Database(_) => APIError::Internal,
			QueryError::DatabaseConnection(e) => e.into(),
			QueryError::SongNotFound(_) => APIError::SongMetadataNotFound,
			QueryError::Vfs(e) => e.into(),
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
		}
	}
}

impl From<user::Error> for APIError {
	fn from(error: user::Error) -> APIError {
		match error {
			user::Error::AuthorizationTokenEncoding => APIError::Internal,
			user::Error::BrancaTokenEncoding => APIError::Internal,
			user::Error::Database(_) => APIError::Internal,
			user::Error::DatabaseConnection(e) => e.into(),
			user::Error::EmptyUsername => APIError::EmptyUsername,
			user::Error::EmptyPassword => APIError::EmptyPassword,
			user::Error::IncorrectUsername => APIError::IncorrectCredentials,
			user::Error::IncorrectPassword => APIError::IncorrectCredentials,
			user::Error::InvalidAuthToken => APIError::IncorrectCredentials,
			user::Error::IncorrectAuthorizationScope => APIError::IncorrectCredentials,
			user::Error::PasswordHashing => APIError::Internal,
			user::Error::MissingLastFMSessionKey => APIError::IncorrectCredentials,
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
		}
	}
}

impl From<ddns::Error> for APIError {
	fn from(error: ddns::Error) -> APIError {
		match error {
			ddns::Error::Database(_) => APIError::Internal,
			ddns::Error::DatabaseConnection(e) => e.into(),
			ddns::Error::UpdateQueryFailed(_) => APIError::Internal,
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

impl From<lastfm::Error> for APIError {
	fn from(error: lastfm::Error) -> APIError {
		match error {
			lastfm::Error::ScrobblerAuthentication(_) => APIError::Internal,
			lastfm::Error::Scrobble(_) => APIError::Internal,
			lastfm::Error::NowPlaying(_) => APIError::Internal,
			lastfm::Error::Query(e) => e.into(),
			lastfm::Error::User(e) => e.into(),
		}
	}
}

impl From<thumbnail::Error> for APIError {
	fn from(error: thumbnail::Error) -> APIError {
		match error {
			thumbnail::Error::EmbeddedArtworkNotFound(_) => APIError::EmbeddedArtworkNotFound,
			thumbnail::Error::Id3(_) => APIError::Internal,
			thumbnail::Error::Image(_) => APIError::Internal,
			thumbnail::Error::Io(_, _) => APIError::Internal,
			thumbnail::Error::Metaflac(_) => APIError::Internal,
			thumbnail::Error::Mp4aMeta(_) => APIError::Internal,
			thumbnail::Error::UnsupportedFormat(_) => APIError::Internal,
		}
	}
}
