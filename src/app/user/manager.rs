use anyhow::*;
use diesel;
use diesel::prelude::*;

use crate::db::users;
use crate::db::DB;

const HASH_ITERATIONS: u32 = 10000;

#[derive(Clone)]
pub struct Manager {
	db: DB,
}

impl Manager {
	pub fn new(db: DB) -> Self {
		Self { db }
	}

	pub fn auth(&self, username: &str, password: &str) -> Result<bool> {
		use crate::db::users::dsl::*;
		let connection = self.db.connect()?;
		match users
			.select(password_hash)
			.filter(name.eq(username))
			.get_result(&connection)
		{
			Err(diesel::result::Error::NotFound) => Ok(false),
			Ok(hash) => {
				let hash: String = hash;
				Ok(verify_password(&hash, password))
			}
			Err(e) => Err(e.into()),
		}
	}

	pub fn count(&self) -> Result<i64> {
		use crate::db::users::dsl::*;
		let connection = self.db.connect()?;
		let count = users.count().get_result(&connection)?;
		Ok(count)
	}

	pub fn exists(&self, username: &str) -> Result<bool> {
		use crate::db::users::dsl::*;
		let connection = self.db.connect()?;
		let results: Vec<String> = users
			.select(name)
			.filter(name.eq(username))
			.get_results(&connection)?;
		Ok(results.len() > 0)
	}

	pub fn is_admin(&self, username: &str) -> Result<bool> {
		use crate::db::users::dsl::*;
		let connection = self.db.connect()?;
		let is_admin: i32 = users
			.filter(name.eq(username))
			.select(admin)
			.get_result(&connection)?;
		Ok(is_admin != 0)
	}

	pub fn lastfm_link(&self, username: &str, lastfm_login: &str, session_key: &str) -> Result<()> {
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

	pub fn get_lastfm_session_key(&self, username: &str) -> Result<String> {
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

	pub fn lastfm_unlink(&self, username: &str) -> Result<()> {
		use crate::db::users::dsl::*;
		let connection = self.db.connect()?;
		diesel::update(users.filter(name.eq(username)))
			.set((lastfm_session_key.eq(""), lastfm_username.eq("")))
			.execute(&connection)?;
		Ok(())
	}
}

#[derive(Debug, Insertable, Queryable)]
#[table_name = "users"]
pub struct User {
	pub name: String,
	pub password_hash: String,
	pub admin: i32,
}

impl User {
	pub fn new(name: &str, password: &str) -> Result<User> {
		let hash = hash_password(password)?;
		Ok(User {
			name: name.to_owned(),
			password_hash: hash,
			admin: 0,
		})
	}
}

pub fn hash_password(password: &str) -> Result<String> {
	match pbkdf2::pbkdf2_simple(password, HASH_ITERATIONS) {
		Ok(hash) => Ok(hash),
		Err(e) => Err(e.into()),
	}
}

fn verify_password(password_hash: &str, attempted_password: &str) -> bool {
	pbkdf2::pbkdf2_check(attempted_password, password_hash).is_ok()
}
