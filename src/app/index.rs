use crate::app::vfs;
use crate::db::DB;

mod query;
#[cfg(test)]
mod test;
mod types;

pub use self::types::*;

#[derive(Clone)]
pub struct Index {
	db: DB,
	vfs_manager: vfs::Manager,
}

impl Index {
	pub fn new(db: DB, vfs_manager: vfs::Manager) -> Self {
		Self { db, vfs_manager }
	}
}
