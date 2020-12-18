use anyhow::anyhow;
use diesel;
use diesel::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

use super::*;
use crate::app::settings::AuthSecret;
use crate::db::DB;

const HASH_ITERATIONS: u32 = 10000;

#[derive(Clone)]
pub struct Manager {
	// TODO make this private and move preferences methods in this file
	pub db: DB,
	auth_secret: AuthSecret,
}

impl Manager {
	pub fn new(db: DB, auth_secret: AuthSecret) -> Self {
		Self { db, auth_secret }
	}

	pub fn create(&self, new_user: &NewUser) -> Result<(), Error> {
		if new_user.name.is_empty() {
			return Err(Error::EmptyUsername);
		}

		let password_hash = hash_password(&new_user.password)?;
		let connection = self.db.connect()?;
		let new_user = User {
			name: new_user.name.to_owned(),
			password_hash,
			admin: new_user.admin as i32,
		};

		diesel::insert_into(users::table)
			.values(&new_user)
			.execute(&connection)
			.map_err(|_| Error::Unspecified)?;
		Ok(())
	}

	pub fn delete(&self, username: &str) -> Result<(), Error> {
		use crate::db::users::dsl::*;
		let connection = self.db.connect()?;
		diesel::delete(users.filter(name.eq(username)))
			.execute(&connection)
			.map_err(|_| Error::Unspecified)?;
		Ok(())
	}

	pub fn set_password(&self, username: &str, password: &str) -> Result<(), Error> {
		let hash = hash_password(password)?;
		let connection = self.db.connect()?;
		use crate::db::users::dsl::*;
		diesel::update(users.filter(name.eq(username)))
			.set(password_hash.eq(hash))
			.execute(&connection)
			.map_err(|_| Error::Unspecified)?;
		Ok(())
	}

	pub fn set_is_admin(&self, username: &str, is_admin: bool) -> Result<(), Error> {
		use crate::db::users::dsl::*;
		let connection = self.db.connect()?;
		diesel::update(users.filter(name.eq(username)))
			.set(admin.eq(is_admin as i32))
			.execute(&connection)
			.map_err(|_| Error::Unspecified)?;
		Ok(())
	}

	pub fn login(&self, username: &str, password: &str) -> Result<AuthToken, Error> {
		use crate::db::users::dsl::*;
		let connection = self.db.connect()?;
		match users
			.select(password_hash)
			.filter(name.eq(username))
			.get_result(&connection)
		{
			Err(diesel::result::Error::NotFound) => Err(Error::IncorrectUsername),
			Ok(hash) => {
				let hash: String = hash;
				if verify_password(&hash, password) {
					self.generate_auth_token(username)
				} else {
					Err(Error::IncorrectPassword)
				}
			}
			Err(_) => Err(Error::Unspecified),
		}
	}

	pub fn authenticate(&self, auth_token: &AuthToken) -> Result<String, Error> {
		let username = self.decode_auth_token(auth_token)?;
		if self.exists(&username)? {
			Ok(username)
		} else {
			Err(Error::IncorrectUsername)
		}
	}

	fn decode_auth_token(&self, auth_token: &AuthToken) -> Result<String, Error> {
		let username = branca::decode(&auth_token.data, &self.auth_secret.key, 0)
			.map_err(|_| Error::Unspecified)?;
		std::str::from_utf8(&username[..])
			.map_err(|_| Error::Unspecified)
			.map(|s| s.to_owned())
	}

	fn generate_auth_token(&self, username: &str) -> Result<AuthToken, Error> {
		branca::encode(
			&username[..].as_bytes(),
			&self.auth_secret.key,
			SystemTime::now()
				.duration_since(UNIX_EPOCH)
				.map_err(|_| Error::Unspecified)?
				.as_secs() as u32,
		)
		.map_err(|_| Error::Unspecified)
		.map(|data| AuthToken { data })
	}

	pub fn count(&self) -> anyhow::Result<i64> {
		use crate::db::users::dsl::*;
		let connection = self.db.connect()?;
		let count = users.count().get_result(&connection)?;
		Ok(count)
	}

	pub fn list(&self) -> Result<Vec<User>, Error> {
		use crate::db::users::dsl::*;
		let connection = self.db.connect()?;
		users
			.select((name, password_hash, admin))
			.get_results(&connection)
			.map_err(|_| Error::Unspecified)
	}

	pub fn exists(&self, username: &str) -> Result<bool, Error> {
		use crate::db::users::dsl::*;
		let connection = self.db.connect()?;
		let results: Vec<String> = users
			.select(name)
			.filter(name.eq(username))
			.get_results(&connection)
			.map_err(|_| Error::Unspecified)?;
		Ok(results.len() > 0)
	}

	pub fn is_admin(&self, username: &str) -> anyhow::Result<bool> {
		use crate::db::users::dsl::*;
		let connection = self.db.connect()?;
		let is_admin: i32 = users
			.filter(name.eq(username))
			.select(admin)
			.get_result(&connection)?;
		Ok(is_admin != 0)
	}

	pub fn lastfm_link(
		&self,
		username: &str,
		lastfm_login: &str,
		session_key: &str,
	) -> anyhow::Result<()> {
		use crate::db::users::dsl::*;
		let connection = self.db.connect()?;
		diesel::update(users.filter(name.eq(username)))
			.set((
				lastfm_username.eq(lastfm_login),
				lastfm_session_key.eq(session_key),
			))
			.execute(&connection)?;
		Ok(())
	}

	pub fn get_lastfm_session_key(&self, username: &str) -> anyhow::Result<String> {
		use crate::db::users::dsl::*;
		let connection = self.db.connect()?;
		let token = users
			.filter(name.eq(username))
			.select(lastfm_session_key)
			.get_result(&connection)?;
		match token {
			Some(t) => Ok(t),
			_ => Err(anyhow!("Missing LastFM credentials")),
		}
	}

	pub fn is_lastfm_linked(&self, username: &str) -> bool {
		self.get_lastfm_session_key(username).is_ok()
	}

	pub fn lastfm_unlink(&self, username: &str) -> anyhow::Result<()> {
		use crate::db::users::dsl::*;
		let connection = self.db.connect()?;
		let null: Option<String> = None;
		diesel::update(users.filter(name.eq(username)))
			.set((lastfm_session_key.eq(&null), lastfm_username.eq(&null)))
			.execute(&connection)?;
		Ok(())
	}
}

fn hash_password(password: &str) -> Result<String, Error> {
	if password.is_empty() {
		return Err(Error::EmptyPassword);
	}
	pbkdf2::pbkdf2_simple(password, HASH_ITERATIONS).map_err(|_| Error::Unspecified)
}

fn verify_password(password_hash: &str, attempted_password: &str) -> bool {
	pbkdf2::pbkdf2_check(attempted_password, password_hash).is_ok()
}
