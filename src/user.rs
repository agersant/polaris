use core::ops::Deref;
use diesel;
use diesel::prelude::*;
use rand;
use ring::{digest, pbkdf2};

use db::ConnectionSource;
use db::users;
use errors::*;

#[derive(Debug, Insertable, Queryable)]
#[table_name="users"]
pub struct User {
	pub name: String,
	pub password_salt: Vec<u8>,
	pub password_hash: Vec<u8>,
	pub admin: i32,
}

static DIGEST_ALG: &'static pbkdf2::PRF = &pbkdf2::HMAC_SHA256;
const CREDENTIAL_LEN: usize = digest::SHA256_OUTPUT_LEN;
const HASH_ITERATIONS: u32 = 10000;
type PasswordHash = [u8; CREDENTIAL_LEN];

impl User {
	pub fn new(name: &str, password: &str, admin: bool) -> User {
		let salt = rand::random::<[u8; 16]>().to_vec();
		let hash = User::hash_password(&salt, password);
		User {
			name: name.to_owned(),
			password_salt: salt,
			password_hash: hash,
			admin: admin as i32,
		}
	}

	pub fn verify_password(&self, attempted_password: &str) -> bool {
		pbkdf2::verify(DIGEST_ALG,
		               HASH_ITERATIONS,
		               &self.password_salt,
		               attempted_password.as_bytes(),
		               &self.password_hash)
				.is_ok()
	}

	fn hash_password(salt: &Vec<u8>, password: &str) -> Vec<u8> {
		let mut hash: PasswordHash = [0; CREDENTIAL_LEN];
		pbkdf2::derive(DIGEST_ALG,
		               HASH_ITERATIONS,
		               salt,
		               password.as_bytes(),
		               &mut hash);
		hash.to_vec()
	}
}

pub fn auth<T>(db: &T, username: &str, password: &str) -> Result<bool>
	where T: ConnectionSource
{
	use db::users::dsl::*;
	let connection = db.get_connection();
	let user: QueryResult<User> = users
		.select((name, password_salt, password_hash, admin))
		.filter(name.eq(username))
		.get_result(connection.deref());
	match user {
		Err(diesel::result::Error::NotFound) => Ok(false),
		Ok(u) => Ok(u.verify_password(password)),
		Err(e) => Err(e.into()),
	}
}

pub fn count<T>(db: &T) -> Result<i64>
	where T: ConnectionSource
{
	use db::users::dsl::*;
	let connection = db.get_connection();
	let count = users.count().get_result(connection.deref())?;
    Ok(count)
}

pub fn is_admin<T>(db: &T, username: &str) -> Result<bool>
	where T: ConnectionSource
{
	use db::users::dsl::*;
	let connection = db.get_connection();
	let is_admin: i32 = users
		.filter(name.eq(username))
		.select(admin)
		.get_result(connection.deref())?;
	Ok(is_admin != 0)
}
