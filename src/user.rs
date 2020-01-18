use anyhow::*;
use diesel;
use diesel::prelude::*;

use crate::db::users;
use crate::db::DB;

#[derive(Debug, Insertable, Queryable)]
#[table_name = "users"]
pub struct User {
	pub name: String,
	pub password_hash: String,
	pub admin: i32,
}

const HASH_ITERATIONS: u32 = 10000;

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

pub fn auth(db: &DB, username: &str, password: &str) -> Result<bool> {
	use crate::db::users::dsl::*;
	let connection = db.connect()?;
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

pub fn count(db: &DB) -> Result<i64> {
	use crate::db::users::dsl::*;
	let connection = db.connect()?;
	let count = users.count().get_result(&connection)?;
	Ok(count)
}

pub fn exists(db: &DB, username: &str) -> Result<bool> {
	use crate::db::users::dsl::*;
	let connection = db.connect()?;
	let results: Vec<String> = users
		.select(name)
		.filter(name.eq(username))
		.get_results(&connection)?;
	Ok(results.len() > 0)
}

pub fn is_admin(db: &DB, username: &str) -> Result<bool> {
	use crate::db::users::dsl::*;
	let connection = db.connect()?;
	let is_admin: i32 = users
		.filter(name.eq(username))
		.select(admin)
		.get_result(&connection)?;
	Ok(is_admin != 0)
}

pub fn lastfm_link(db: &DB, username: &str, lastfm_login: &str, session_key: &str) -> Result<()> {
	use crate::db::users::dsl::*;
	let connection = db.connect()?;
	diesel::update(users.filter(name.eq(username)))
		.set((
			lastfm_username.eq(lastfm_login),
			lastfm_session_key.eq(session_key),
		))
		.execute(&connection)?;
	Ok(())
}

pub fn get_lastfm_session_key(db: &DB, username: &str) -> Result<String> {
	use crate::db::users::dsl::*;
	let connection = db.connect()?;
	let token = users
		.filter(name.eq(username))
		.select(lastfm_session_key)
		.get_result(&connection)?;
	match token {
		Some(t) => Ok(t),
		_ => Err(anyhow!("Missing LastFM credentials")),
	}
}

pub fn is_lastfm_linked(db: &DB, username: &str) -> bool {
	get_lastfm_session_key(db, username).is_ok()
}

pub fn lastfm_unlink(db: &DB, username: &str) -> Result<()> {
	use crate::db::users::dsl::*;
	let connection = db.connect()?;
	diesel::update(users.filter(name.eq(username)))
		.set((lastfm_session_key.eq(""), lastfm_username.eq("")))
		.execute(&connection)?;
	Ok(())
}
