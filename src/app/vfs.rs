use anyhow::{bail, Result};
use core::ops::Deref;
use diesel::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{self, Path, PathBuf};

use crate::db::{mount_points, DB};

#[cfg(test)]
mod test;

#[derive(Clone, Debug, Deserialize, Insertable, PartialEq, Eq, Queryable, Serialize)]
#[diesel(table_name = mount_points)]
pub struct MountDir {
	pub source: String,
	pub name: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Mount {
	pub source: PathBuf,
	pub name: String,
}

impl From<MountDir> for Mount {
	fn from(m: MountDir) -> Self {
		let separator_regex = Regex::new(r"\\|/").unwrap();
		let mut correct_separator = String::new();
		correct_separator.push(path::MAIN_SEPARATOR);
		let path_string = separator_regex.replace_all(&m.source, correct_separator.as_str());
		let source = PathBuf::from(path_string.deref());
		Self {
			name: m.name,
			source,
		}
	}
}

#[allow(clippy::upper_case_acronyms)]
pub struct VFS {
	mounts: Vec<Mount>,
}

impl VFS {
	pub fn new(mounts: Vec<Mount>) -> VFS {
		VFS { mounts }
	}

	pub fn real_to_virtual<P: AsRef<Path>>(&self, real_path: P) -> Result<PathBuf> {
		for mount in &self.mounts {
			if let Ok(p) = real_path.as_ref().strip_prefix(&mount.source) {
				let mount_path = Path::new(&mount.name);
				return if p.components().count() == 0 {
					Ok(mount_path.to_path_buf())
				} else {
					Ok(mount_path.join(p))
				};
			}
		}
		bail!("Real path has no match in VFS")
	}

	pub fn virtual_to_real<P: AsRef<Path>>(&self, virtual_path: P) -> Result<PathBuf> {
		for mount in &self.mounts {
			let mount_path = Path::new(&mount.name);
			if let Ok(p) = virtual_path.as_ref().strip_prefix(mount_path) {
				return if p.components().count() == 0 {
					Ok(mount.source.clone())
				} else {
					Ok(mount.source.join(p))
				};
			}
		}
		bail!("Virtual path has no match in VFS")
	}

	pub fn mounts(&self) -> &Vec<Mount> {
		&self.mounts
	}
}

#[derive(Clone)]
pub struct Manager {
	db: DB,
}

impl Manager {
	pub fn new(db: DB) -> Self {
		Self { db }
	}

	pub fn get_vfs(&self) -> Result<VFS> {
		let mount_dirs = self.mount_dirs()?;
		let mounts = mount_dirs.into_iter().map(|p| p.into()).collect();
		Ok(VFS::new(mounts))
	}

	pub fn mount_dirs(&self) -> Result<Vec<MountDir>> {
		use self::mount_points::dsl::*;
		let mut connection = self.db.connect()?;
		let mount_dirs: Vec<MountDir> = mount_points
			.select((source, name))
			.get_results(&mut connection)?;
		Ok(mount_dirs)
	}

	pub fn set_mount_dirs(&self, mount_dirs: &[MountDir]) -> Result<()> {
		use self::mount_points::dsl::*;
		let mut connection = self.db.connect()?;
		diesel::delete(mount_points).execute(&mut connection)?;
		diesel::insert_into(mount_points)
			.values(mount_dirs)
			.execute(&mut *connection)?; // TODO https://github.com/diesel-rs/diesel/issues/1822
		Ok(())
	}
}
