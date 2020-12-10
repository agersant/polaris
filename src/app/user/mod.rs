use crate::db::users;

mod manager;
mod preferences;

pub use manager::*;
pub use preferences::*;

#[derive(Debug, Insertable, Queryable)]
#[table_name = "users"]
pub struct User {
	pub name: String,
	pub password_hash: String,
	pub admin: i32,
}

impl User {
	pub fn new(name: &str, password: &str) -> anyhow::Result<User> {
		let hash = hash_password(password)?;
		Ok(User {
			name: name.to_owned(),
			password_hash: hash,
			admin: 0,
		})
	}
}
