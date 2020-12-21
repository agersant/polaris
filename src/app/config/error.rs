#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("Unspecified")]
	Unspecified,
}

impl From<anyhow::Error> for Error {
	fn from(_: anyhow::Error) -> Self {
		Error::Unspecified
	}
}
