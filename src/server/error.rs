use std::path::PathBuf;
use thiserror::Error;

use crate::app;

#[derive(Error, Debug)]
pub enum APIError {
	#[error("Could not read API version header")]
	InvalidAPIVersionHeader,
	#[error("Could not parse API version header")]
	APIVersionHeaderParseError,
	#[error("Unsupported API version")]
	UnsupportedAPIVersion,
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
	Database(sqlx::Error),
	#[error("Directory not found: {0}")]
	DirectoryNotFound(PathBuf),
	#[error("Artist not found")]
	ArtistNotFound,
	#[error("Album not found")]
	AlbumNotFound,
	#[error("Genre not found")]
	GenreNotFound,
	#[error("Song not found")]
	SongNotFound,
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
	#[error("Could not parse search query")]
	SearchQueryParseError,
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
	#[error("Unsupported thumbnail format: `{0}`")]
	UnsupportedThumbnailFormat(&'static str),
	#[error("Audio decoding error: `{0}`")]
	AudioDecoding(symphonia::core::errors::Error),
	#[error("Empty audio file: `{0}`")]
	AudioEmpty(PathBuf),
	#[error("User not found")]
	UserNotFound,
	#[error("Path not found in virtual filesystem")]
	VFSPathNotFound,
}

impl From<app::Error> for APIError {
	fn from(error: app::Error) -> APIError {
		match error {
			app::Error::ThreadPoolBuilder(_) => APIError::Internal,
			app::Error::ThreadJoining(_) => APIError::Internal,

			app::Error::Io(p, e) => APIError::Io(p, e),
			app::Error::Ape(_) => APIError::Internal,
			app::Error::Id3(p, e) => APIError::ThumbnailId3Decoding(p, e),
			app::Error::Metaflac(p, e) => APIError::ThumbnailFlacDecoding(p, e),
			app::Error::Mp4aMeta(p, e) => APIError::ThumbnailMp4Decoding(p, e),
			app::Error::Opus(_) => APIError::Internal,
			app::Error::Vorbis(_) => APIError::Internal,
			app::Error::VorbisCommentNotFoundInFlacFile => APIError::Internal,
			app::Error::Image(p, e) => APIError::ThumbnailImageDecoding(p, e),
			app::Error::UnsupportedFormat(f) => APIError::UnsupportedThumbnailFormat(f),

			app::Error::MediaEmpty(p) => APIError::AudioEmpty(p),
			app::Error::MediaDecodeError(e) => APIError::AudioDecoding(e),
			app::Error::MediaDecoderError(e) => APIError::AudioDecoding(e),
			app::Error::MediaPacketError(e) => APIError::AudioDecoding(e),
			app::Error::MediaProbeError(e) => APIError::AudioDecoding(e),

			app::Error::PeaksSerialization(_) => APIError::Internal,
			app::Error::PeaksDeserialization(_) => APIError::Internal,

			app::Error::Database(e) => APIError::Database(e),
			app::Error::ConnectionPoolBuild => APIError::Internal,
			app::Error::ConnectionPool => APIError::Internal,
			app::Error::Migration(_) => APIError::Internal,

			app::Error::UpdateQueryFailed(s) => APIError::DdnsUpdateQueryFailed(s),
			app::Error::UpdateQueryTransport => APIError::DdnsUpdateQueryFailed(0),

			app::Error::AuthenticationSecretNotFound => APIError::Internal,
			app::Error::AuthenticationSecretInvalid => APIError::Internal,
			app::Error::MiscSettingsNotFound => APIError::Internal,
			app::Error::IndexAlbumArtPatternInvalid => APIError::Internal,

			app::Error::Toml(_) => APIError::Internal,
			app::Error::IndexDeserializationError => APIError::Internal,
			app::Error::IndexSerializationError => APIError::Internal,

			app::Error::CouldNotMapToRealPath(_) => APIError::VFSPathNotFound,
			app::Error::UserNotFound => APIError::UserNotFound,
			app::Error::DirectoryNotFound(d) => APIError::DirectoryNotFound(d),
			app::Error::ArtistNotFound => APIError::ArtistNotFound,
			app::Error::AlbumNotFound => APIError::AlbumNotFound,
			app::Error::GenreNotFound => APIError::GenreNotFound,
			app::Error::SongNotFound => APIError::SongNotFound,
			app::Error::PlaylistNotFound => APIError::PlaylistNotFound,
			app::Error::SearchQueryParseError => APIError::SearchQueryParseError,
			app::Error::EmbeddedArtworkNotFound(_) => APIError::EmbeddedArtworkNotFound,

			app::Error::EmptyUsername => APIError::EmptyUsername,
			app::Error::EmptyPassword => APIError::EmptyPassword,
			app::Error::IncorrectUsername => APIError::IncorrectCredentials,
			app::Error::IncorrectPassword => APIError::IncorrectCredentials,
			app::Error::InvalidAuthToken => APIError::IncorrectCredentials,
			app::Error::IncorrectAuthorizationScope => APIError::IncorrectCredentials,
			app::Error::MissingLastFMSessionKey => APIError::IncorrectCredentials,
			app::Error::PasswordHashing => APIError::PasswordHashing,
			app::Error::AuthorizationTokenEncoding => APIError::AuthorizationTokenEncoding,
			app::Error::BrancaTokenEncoding => APIError::BrancaTokenEncoding,

			app::Error::ScrobblerAuthentication(e) => APIError::LastFMScrobblerAuthentication(e),
			app::Error::Scrobble(e) => APIError::LastFMScrobble(e),
			app::Error::NowPlaying(e) => APIError::LastFMNowPlaying(e),
		}
	}
}