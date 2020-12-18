#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum Error {
	#[error("Cannot use empty username")]
	EmptyUsername,
	#[error("Cannot use empty password")]
	EmptyPassword,
	#[error("Username does not exist")]
	IncorrectUsername,
	#[error("Password does not match username")]
	IncorrectPassword,
	#[error("Unspecified")]
	Unspecified,
}

impl From<anyhow::Error> for Error {
	fn from(_: anyhow::Error) -> Self {
		Error::Unspecified
	}
}
