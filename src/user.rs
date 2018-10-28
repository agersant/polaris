use core::ops::Deref;
use diesel;
use diesel::prelude::*;
use rand;
use ring::{digest, pbkdf2};

use db::users;
use db::ConnectionSource;
use errors::*;

#[derive(Debug, Insertable, Queryable)]
#[table_name = "users"]
pub struct User {
	pub name: String,
	pub password_salt: Vec<u8>,
	pub password_hash: Vec<u8>,
	pub admin: i32,
}

static DIGEST_ALG: &'static digest::Algorithm = &digest::SHA256;
const CREDENTIAL_LEN: usize = digest::SHA256_OUTPUT_LEN;
const HASH_ITERATIONS: u32 = 10000;
type PasswordHash = [u8; CREDENTIAL_LEN];

impl User {
	pub fn new(name: &str, password: &str) -> User {
		let salt = rand::random::<[u8; 16]>().to_vec();
		let hash = hash_password(&salt, password);
		User {
			name: name.to_owned(),
			password_salt: salt,
			password_hash: hash,
			admin: 0,
		}
	}
}

pub fn hash_password(salt: &[u8], password: &str) -> Vec<u8> {
	let mut hash: PasswordHash = [0; CREDENTIAL_LEN];
	pbkdf2::derive(
		DIGEST_ALG,
		HASH_ITERATIONS,
		salt,
		password.as_bytes(),
		&mut hash,
	);
	hash.to_vec()
}

fn verify_password(
	password_hash: &Vec<u8>,
	password_salt: &Vec<u8>,
	attempted_password: &str,
) -> bool {
	pbkdf2::verify(
		DIGEST_ALG,
		HASH_ITERATIONS,
		password_salt,
		attempted_password.as_bytes(),
		password_hash,
	)
	.is_ok()
}

pub fn auth<T>(db: &T, username: &str, password: &str) -> Result<bool>
where
	T: ConnectionSource,
{
	use db::users::dsl::*;
	let connection = db.get_connection();
	match users
		.select((password_hash, password_salt))
		.filter(name.eq(username))
		.get_result(connection.deref())
	{
		Err(diesel::result::Error::NotFound) => Ok(false),
		Ok((hash, salt)) => Ok(verify_password(&hash, &salt, password)),
		Err(e) => Err(e.into()),
	}
}

pub fn count<T>(db: &T) -> Result<i64>
where
	T: ConnectionSource,
{
	use db::users::dsl::*;
	let connection = db.get_connection();
	let count = users.count().get_result(connection.deref())?;
	Ok(count)
}

pub fn is_admin<T>(db: &T, username: &str) -> Result<bool>
where
	T: ConnectionSource,
{
	use db::users::dsl::*;
	let connection = db.get_connection();
	let is_admin: i32 = users
		.filter(name.eq(username))
		.select(admin)
		.get_result(connection.deref())?;
	Ok(is_admin != 0)
}

pub fn lastfm_link<T>(db: &T, username: &str, lastfm_login: &str, session_key: &str) -> Result<()>
where
	T: ConnectionSource,
{
	use db::users::dsl::*;
	let connection = db.get_connection();
	diesel::update(users.filter(name.eq(username)))
		.set((
			lastfm_username.eq(lastfm_login),
			lastfm_session_key.eq(session_key),
		))
		.execute(connection.deref())?;
	Ok(())
}

pub fn get_lastfm_session_key<T>(db: &T, username: &str) -> Result<String>
where
	T: ConnectionSource,
{
	use db::users::dsl::*;
	let connection = db.get_connection();
	let token = users
		.filter(name.eq(username))
		.select(lastfm_session_key)
		.get_result(connection.deref())?;
	match token {
		Some(t) => Ok(t),
		_ => bail!(ErrorKind::MissingLastFMCredentials),
	}
}

pub fn lastfm_unlink<T>(db: &T, username: &str) -> Result<()>
where
	T: ConnectionSource,
{
	use db::users::dsl::*;
	let connection = db.get_connection();
	diesel::update(users.filter(name.eq(username)))
		.set((lastfm_session_key.eq(""), lastfm_username.eq("")))
		.execute(connection.deref())?;
	Ok(())
}
