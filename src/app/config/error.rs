#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("Missing auth secret")]
	AuthSecretNotFound,
	#[error("Missing index sleep duration")]
	IndexSleepDurationNotFound,
	#[error("Missing index album art pattern")]
	IndexAlbumArtPatternNotFound,
	#[error("Index album art pattern is not a valid regex")]
	IndexAlbumArtPatternInvalid,
	#[error("Unspecified")]
	Unspecified,
}

impl From<anyhow::Error> for Error {
	fn from(_: anyhow::Error) -> Self {
		Error::Unspecified
	}
}
