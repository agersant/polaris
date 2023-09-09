use std::path::PathBuf;
use thiserror::Error;

use crate::app::index::QueryError;
use crate::app::{config, ddns, lastfm, playlist, settings, thumbnail, user, vfs};
use crate::db;

#[derive(Error, Debug)]
pub enum APIError {
	#[error("Could not encode authorization token")]
	AuthorizationTokenEncoding,
	#[error("Administrator permission is required")]
	AdminPermissionRequired,
	#[error("Audio file could not be opened")]
	AudioFileIOError,
	#[error("Authentication is required")]
	AuthenticationRequired,
	#[error("Could not encode Branca token")]
	BrancaTokenEncoding,
	#[error("Database error:\n\n{0}")]
	Database(diesel::result::Error),
	#[error("DDNS update query failed with HTTP status {0}")]
	DdnsUpdateQueryFailed(u16),
	#[error("Cannot delete your own account")]
	DeletingOwnAccount,
	#[error("EmbeddedArtworkNotFound")]
	EmbeddedArtworkNotFound,
	#[error("EmptyUsername")]
	EmptyUsername,
	#[error("EmptyPassword")]
	EmptyPassword,
	#[error("Incorrect Credentials")]
	IncorrectCredentials,
	#[error("No last.fm account has been linked")]
	LastFMAccountNotLinked,
	#[error("Could not decode content as base64 after linking last.fm account")]
	LastFMLinkContentBase64DecodeError,
	#[error("Could not decode content as UTF-8 after linking last.fm account")]
	LastFMLinkContentEncodingError,
	#[error("Could send Now Playing update to last.fm:\n\n{0}")]
	LastFMNowPlaying(rustfm_scrobble::ScrobblerError),
	#[error("Could emit scrobble with last.fm:\n\n{0}")]
	LastFMScrobble(rustfm_scrobble::ScrobblerError),
	#[error("Could authenticate with last.fm:\n\n{0}")]
	LastFMScrobblerAuthentication(rustfm_scrobble::ScrobblerError),
	#[error("Internal server error")]
	Internal,
	#[error("File I/O error for `{0}`:\n\n{1}")]
	Io(PathBuf, std::io::Error),
	#[error("Cannot remove your own admin privilege")]
	OwnAdminPrivilegeRemoval,
	#[error("Could not hash password")]
	PasswordHashing,
	#[error("Playlist not found")]
	PlaylistNotFound,
	#[error("Settings error:\n\n{0}")]
	Settings(settings::Error),
	#[error("Song not found")]
	SongMetadataNotFound,
	#[error("Could not decode thumbnail from flac file `{0}`:\n\n{1}")]
	ThumbnailFlacDecoding(PathBuf, metaflac::Error),
	#[error("Thumbnail file could not be opened")]
	ThumbnailFileIOError,
	#[error("Could not decode thumbnail from ID3 tag in `{0}`:\n\n{1}")]
	ThumbnailId3Decoding(PathBuf, id3::Error),
	#[error("Could not decode image thumbnail in `{0}`:\n\n{1}")]
	ThumbnailImageDecoding(PathBuf, image::error::ImageError),
	#[error("Could not decode thumbnail from mp4 file `{0}`:\n\n{1}")]
	ThumbnailMp4Decoding(PathBuf, mp4ameta::Error),
	#[error("Toml deserialization error:\n\n{0}")]
	TomlDeserialization(toml::de::Error),
	#[error("Unsupported thumbnail format: `{0}`")]
	UnsupportedThumbnailFormat(&'static str),
	#[error("User not found")]
	UserNotFound,
	#[error("Path not found in virtual filesystem")]
	VFSPathNotFound,
}

impl From<config::Error> for APIError {
	fn from(error: config::Error) -> APIError {
		match error {
			config::Error::Ddns(e) => e.into(),
			config::Error::Io(p, e) => APIError::Io(p, e),
			config::Error::Settings(e) => e.into(),
			config::Error::Toml(e) => APIError::TomlDeserialization(e),
			config::Error::User(e) => e.into(),
			config::Error::Vfs(e) => e.into(),
		}
	}
}

impl From<playlist::Error> for APIError {
	fn from(error: playlist::Error) -> APIError {
		match error {
			playlist::Error::Database(e) => APIError::Database(e),
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
			QueryError::Database(e) => APIError::Database(e),
			QueryError::DatabaseConnection(e) => e.into(),
			QueryError::SongNotFound(_) => APIError::SongMetadataNotFound,
			QueryError::Vfs(e) => e.into(),
		}
	}
}

impl From<settings::Error> for APIError {
	fn from(error: settings::Error) -> APIError {
		match error {
			settings::Error::AuthenticationSecretNotFound => APIError::Settings(error),
			settings::Error::DatabaseConnection(e) => e.into(),
			settings::Error::AuthenticationSecretInvalid => APIError::Settings(error),
			settings::Error::MiscSettingsNotFound => APIError::Settings(error),
			settings::Error::IndexAlbumArtPatternInvalid => APIError::Settings(error),
			settings::Error::Database(e) => APIError::Database(e),
		}
	}
}

impl From<user::Error> for APIError {
	fn from(error: user::Error) -> APIError {
		match error {
			user::Error::AuthorizationTokenEncoding => APIError::AuthorizationTokenEncoding,
			user::Error::BrancaTokenEncoding => APIError::BrancaTokenEncoding,
			user::Error::Database(e) => APIError::Database(e),
			user::Error::DatabaseConnection(e) => e.into(),
			user::Error::EmptyPassword => APIError::EmptyPassword,
			user::Error::EmptyUsername => APIError::EmptyUsername,
			user::Error::IncorrectAuthorizationScope => APIError::IncorrectCredentials,
			user::Error::IncorrectPassword => APIError::IncorrectCredentials,
			user::Error::IncorrectUsername => APIError::IncorrectCredentials,
			user::Error::InvalidAuthToken => APIError::IncorrectCredentials,
			user::Error::MissingLastFMSessionKey => APIError::IncorrectCredentials,
			user::Error::PasswordHashing => APIError::PasswordHashing,
		}
	}
}

impl From<vfs::Error> for APIError {
	fn from(error: vfs::Error) -> APIError {
		match error {
			vfs::Error::CouldNotMapToVirtualPath(_) => APIError::VFSPathNotFound,
			vfs::Error::CouldNotMapToRealPath(_) => APIError::VFSPathNotFound,
			vfs::Error::Database(e) => APIError::Database(e),
			vfs::Error::DatabaseConnection(e) => e.into(),
		}
	}
}

impl From<ddns::Error> for APIError {
	fn from(error: ddns::Error) -> APIError {
		match error {
			ddns::Error::Database(e) => APIError::Database(e),
			ddns::Error::DatabaseConnection(e) => e.into(),
			ddns::Error::UpdateQueryFailed(s) => APIError::DdnsUpdateQueryFailed(s),
			ddns::Error::UpdateQueryTransport => APIError::DdnsUpdateQueryFailed(0),
		}
	}
}

impl From<db::Error> for APIError {
	fn from(error: db::Error) -> APIError {
		match error {
			db::Error::ConnectionPoolBuild => APIError::Internal,
			db::Error::ConnectionPool => APIError::Internal,
			db::Error::Io(p, e) => APIError::Io(p, e),
			db::Error::Migration => APIError::Internal,
		}
	}
}

impl From<lastfm::Error> for APIError {
	fn from(error: lastfm::Error) -> APIError {
		match error {
			lastfm::Error::ScrobblerAuthentication(e) => APIError::LastFMScrobblerAuthentication(e),
			lastfm::Error::Scrobble(e) => APIError::LastFMScrobble(e),
			lastfm::Error::NowPlaying(e) => APIError::LastFMNowPlaying(e),
			lastfm::Error::Query(e) => e.into(),
			lastfm::Error::User(e) => e.into(),
		}
	}
}

impl From<thumbnail::Error> for APIError {
	fn from(error: thumbnail::Error) -> APIError {
		match error {
			thumbnail::Error::EmbeddedArtworkNotFound(_) => APIError::EmbeddedArtworkNotFound,
			thumbnail::Error::Id3(p, e) => APIError::ThumbnailId3Decoding(p, e),
			thumbnail::Error::Image(p, e) => APIError::ThumbnailImageDecoding(p, e),
			thumbnail::Error::Io(p, e) => APIError::Io(p, e),
			thumbnail::Error::Metaflac(p, e) => APIError::ThumbnailFlacDecoding(p, e),
			thumbnail::Error::Mp4aMeta(p, e) => APIError::ThumbnailMp4Decoding(p, e),
			thumbnail::Error::UnsupportedFormat(f) => APIError::UnsupportedThumbnailFormat(f),
		}
	}
}
