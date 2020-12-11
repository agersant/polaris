use anyhow::*;
use diesel::prelude::*;
use std::path::Path;

use super::*;
use crate::db::mount_points;
use crate::db::DB;

#[derive(Clone)]
pub struct Manager {
	db: DB,
}

impl Manager {
	pub fn new(db: DB) -> Self {
		Self { db }
	}

	pub fn get_vfs(&self) -> Result<VFS> {
		use self::mount_points::dsl::*;
		let mut vfs = VFS::new();
		let connection = self.db.connect()?;
		let points: Vec<MountPoint> = mount_points
			.select((source, name))
			.get_results(&connection)?;
		for point in points {
			vfs.mount(&Path::new(&point.source), &point.name)?;
		}
		Ok(vfs)
	}
}
