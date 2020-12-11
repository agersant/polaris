use crate::db::users;

mod error;
mod manager;
mod preferences;
#[cfg(test)]
mod test;

pub use error::*;
pub use manager::*;
pub use preferences::*;

#[derive(Debug, Insertable, Queryable)]
#[table_name = "users"]
pub struct User {
	pub name: String,
	pub password_hash: String,
	pub admin: i32,
}
