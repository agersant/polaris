use anyhow::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use crate::db::mount_points;

mod manager;
#[cfg(test)]
mod test;

pub use manager::*;

#[derive(Clone, Debug, Deserialize, Insertable, PartialEq, Queryable, Serialize)]
#[table_name = "mount_points"]
pub struct MountPoint {
	pub source: String,
	pub name: String,
}

pub struct VFS {
	mount_points: HashMap<String, PathBuf>,
}

impl VFS {
	pub fn new() -> VFS {
		VFS {
			mount_points: HashMap::new(),
		}
	}

	pub fn mount(&mut self, real_path: &Path, name: &str) -> Result<()> {
		self.mount_points
			.insert(name.to_owned(), real_path.to_path_buf());
		Ok(())
	}

	pub fn real_to_virtual<P: AsRef<Path>>(&self, real_path: P) -> Result<PathBuf> {
		for (name, target) in &self.mount_points {
			if let Ok(p) = real_path.as_ref().strip_prefix(target) {
				let mount_path = Path::new(&name);
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
		for (name, target) in &self.mount_points {
			let mount_path = Path::new(&name);
			if let Ok(p) = virtual_path.as_ref().strip_prefix(mount_path) {
				return if p.components().count() == 0 {
					Ok(target.clone())
				} else {
					Ok(target.join(p))
				};
			}
		}
		bail!("Virtual path has no match in VFS")
	}

	pub fn get_mount_points(&self) -> &HashMap<String, PathBuf> {
		&self.mount_points
	}
}
