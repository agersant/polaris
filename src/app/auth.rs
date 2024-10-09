use std::time::{SystemTime, UNIX_EPOCH};

use pbkdf2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use pbkdf2::Pbkdf2;
use rand::rngs::OsRng;

use serde::{Deserialize, Serialize};

use crate::app::Error;

#[derive(Clone, Default)]
pub struct Secret(pub [u8; 32]);

impl AsRef<[u8]> for Secret {
	fn as_ref(&self) -> &[u8] {
		&self.0
	}
}

impl AsMut<[u8]> for Secret {
	fn as_mut(&mut self) -> &mut [u8] {
		&mut self.0
	}
}

#[derive(Debug)]
pub struct Token(pub String);

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum Scope {
	PolarisAuth,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Authorization {
	pub username: String,
	pub scope: Scope,
}

pub fn hash_password(password: &str) -> Result<String, Error> {
	if password.is_empty() {
		return Err(Error::EmptyPassword);
	}
	let salt = SaltString::generate(&mut OsRng);
	match Pbkdf2.hash_password(password.as_bytes(), &salt) {
		Ok(h) => Ok(h.to_string()),
		Err(_) => Err(Error::PasswordHashing),
	}
}

pub fn verify_password(password_hash: &str, attempted_password: &str) -> bool {
	match PasswordHash::new(password_hash) {
		Ok(h) => Pbkdf2
			.verify_password(attempted_password.as_bytes(), &h)
			.is_ok(),
		Err(_) => false,
	}
}

pub fn generate_auth_token(
	authorization: &Authorization,
	auth_secret: &Secret,
) -> Result<Token, Error> {
	let serialized_authorization =
		serde_json::to_string(&authorization).or(Err(Error::AuthorizationTokenEncoding))?;
	branca::encode(
		serialized_authorization.as_bytes(),
		auth_secret.as_ref(),
		SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.unwrap_or_default()
			.as_secs() as u32,
	)
	.or(Err(Error::BrancaTokenEncoding))
	.map(Token)
}

pub fn decode_auth_token(
	auth_token: &Token,
	scope: Scope,
	auth_secret: &Secret,
) -> Result<Authorization, Error> {
	let Token(data) = auth_token;
	let ttl = match scope {
		Scope::PolarisAuth => 0, // permanent
	};
	let authorization =
		branca::decode(data, auth_secret.as_ref(), ttl).map_err(|_| Error::InvalidAuthToken)?;
	let authorization: Authorization =
		serde_json::from_slice(&authorization[..]).map_err(|_| Error::InvalidAuthToken)?;
	if authorization.scope != scope {
		return Err(Error::IncorrectAuthorizationScope);
	}
	Ok(authorization)
}
