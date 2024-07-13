use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::server::error::APIError;

impl IntoResponse for APIError {
	fn into_response(self) -> Response {
		let message = self.to_string();
		let status_code = match self {
			APIError::AuthorizationTokenEncoding => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::AdminPermissionRequired => StatusCode::FORBIDDEN,
			APIError::AudioFileIOError => StatusCode::NOT_FOUND,
			APIError::AuthenticationRequired => StatusCode::UNAUTHORIZED,
			APIError::BrancaTokenEncoding => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::DdnsUpdateQueryFailed(s) => {
				StatusCode::from_u16(s).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
			}
			APIError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::DeletingOwnAccount => StatusCode::CONFLICT,
			APIError::EmbeddedArtworkNotFound => StatusCode::NOT_FOUND,
			APIError::EmptyPassword => StatusCode::BAD_REQUEST,
			APIError::EmptyUsername => StatusCode::BAD_REQUEST,
			APIError::IncorrectCredentials => StatusCode::UNAUTHORIZED,
			APIError::Internal => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::Io(_, _) => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::LastFMAccountNotLinked => StatusCode::NO_CONTENT,
			APIError::LastFMLinkContentBase64DecodeError => StatusCode::BAD_REQUEST,
			APIError::LastFMLinkContentEncodingError => StatusCode::BAD_REQUEST,
			APIError::LastFMNowPlaying(_) => StatusCode::FAILED_DEPENDENCY,
			APIError::LastFMScrobble(_) => StatusCode::FAILED_DEPENDENCY,
			APIError::LastFMScrobblerAuthentication(_) => StatusCode::FAILED_DEPENDENCY,
			APIError::OwnAdminPrivilegeRemoval => StatusCode::CONFLICT,
			APIError::PasswordHashing => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::PlaylistNotFound => StatusCode::NOT_FOUND,
			APIError::Settings(_) => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::SongMetadataNotFound => StatusCode::NOT_FOUND,
			APIError::ThumbnailFlacDecoding(_, _) => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::ThumbnailFileIOError => StatusCode::NOT_FOUND,
			APIError::ThumbnailId3Decoding(_, _) => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::ThumbnailImageDecoding(_, _) => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::ThumbnailMp4Decoding(_, _) => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::TomlDeserialization(_) => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::UnsupportedThumbnailFormat(_) => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::UserNotFound => StatusCode::NOT_FOUND,
			APIError::VFSPathNotFound => StatusCode::NOT_FOUND,
		};

		(status_code, message).into_response()
	}
}
