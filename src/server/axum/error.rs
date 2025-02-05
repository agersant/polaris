use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::server::error::APIError;

impl IntoResponse for APIError {
	fn into_response(self) -> Response {
		let message = self.to_string();
		let status_code = match self {
			APIError::InvalidAPIVersionHeader => StatusCode::BAD_REQUEST,
			APIError::APIVersionHeaderParseError => StatusCode::BAD_REQUEST,
			APIError::UnsupportedAPIVersion => StatusCode::NOT_ACCEPTABLE,
			APIError::AuthorizationTokenEncoding => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::AdminPermissionRequired => StatusCode::FORBIDDEN,
			APIError::AudioFileIOError => StatusCode::NOT_FOUND,
			APIError::AuthenticationRequired => StatusCode::UNAUTHORIZED,
			APIError::BrancaTokenEncoding => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::DdnsUpdateQueryFailed(s) => {
				StatusCode::from_u16(s).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
			}
			APIError::NativeDatabase(_) => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::DeletingOwnAccount => StatusCode::CONFLICT,
			APIError::DirectoryNotFound(_) => StatusCode::NOT_FOUND,
			APIError::DuplicateUsername => StatusCode::CONFLICT,
			APIError::ArtistNotFound => StatusCode::NOT_FOUND,
			APIError::AlbumNotFound => StatusCode::NOT_FOUND,
			APIError::GenreNotFound => StatusCode::NOT_FOUND,
			APIError::SongNotFound => StatusCode::NOT_FOUND,
			APIError::EmbeddedArtworkNotFound => StatusCode::NOT_FOUND,
			APIError::EmptyPassword => StatusCode::BAD_REQUEST,
			APIError::EmptyUsername => StatusCode::BAD_REQUEST,
			APIError::IncorrectCredentials => StatusCode::UNAUTHORIZED,
			APIError::Internal => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::InvalidAlbumArtPattern => StatusCode::BAD_REQUEST,
			APIError::InvalidDDNSURL => StatusCode::BAD_REQUEST,
			APIError::Io(_, _) => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::OwnAdminPrivilegeRemoval => StatusCode::CONFLICT,
			APIError::PasswordHashing => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::PlaylistNotFound => StatusCode::NOT_FOUND,
			APIError::SearchQueryParseError => StatusCode::BAD_REQUEST,
			APIError::ThumbnailFlacDecoding(_, _) => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::ThumbnailFileIOError => StatusCode::NOT_FOUND,
			APIError::ThumbnailId3Decoding(_, _) => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::ThumbnailImageDecoding(_, _) => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::ThumbnailMp4Decoding(_, _) => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::UnsupportedThumbnailFormat(_) => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::AudioEmpty(_) => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::AudioDecoding(_) => StatusCode::INTERNAL_SERVER_ERROR,
			APIError::UserNotFound => StatusCode::NOT_FOUND,
			APIError::VFSPathNotFound => StatusCode::NOT_FOUND,
		};

		(status_code, message).into_response()
	}
}
