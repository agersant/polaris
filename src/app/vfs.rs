use anyhow::{bail, Result};
use core::ops::Deref;
use diesel::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{self, Path, PathBuf};

use crate::db::{mount_points, DB};

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
		let mut connection = self.db.connect()?;
		connection
			.transaction::<_, diesel::result::Error, _>(|connection| {
				use self::mount_points::dsl::*;
				diesel::delete(mount_points).execute(&mut *connection)?;
				diesel::insert_into(mount_points)
					.values(mount_dirs)
					.execute(&mut *connection)?; // TODO https://github.com/diesel-rs/diesel/issues/1822
				Ok(())
			})
			.map_err(anyhow::Error::new)?;
		Ok(())
	}
}

#[cfg(test)]
mod test {

	use super::*;

	#[test]
	fn converts_virtual_to_real() {
		let vfs = VFS::new(vec![Mount {
			name: "root".to_owned(),
			source: Path::new("test_dir").to_owned(),
		}]);
		let real_path: PathBuf = ["test_dir", "somewhere", "something.png"].iter().collect();
		let virtual_path: PathBuf = ["root", "somewhere", "something.png"].iter().collect();
		let converted_path = vfs.virtual_to_real(virtual_path.as_path()).unwrap();
		assert_eq!(converted_path, real_path);
	}

	#[test]
	fn converts_virtual_to_real_top_level() {
		let vfs = VFS::new(vec![Mount {
			name: "root".to_owned(),
			source: Path::new("test_dir").to_owned(),
		}]);
		let real_path = Path::new("test_dir");
		let converted_path = vfs.virtual_to_real(Path::new("root")).unwrap();
		assert_eq!(converted_path, real_path);
	}

	#[test]
	fn converts_real_to_virtual() {
		let vfs = VFS::new(vec![Mount {
			name: "root".to_owned(),
			source: Path::new("test_dir").to_owned(),
		}]);
		let virtual_path: PathBuf = ["root", "somewhere", "something.png"].iter().collect();
		let real_path: PathBuf = ["test_dir", "somewhere", "something.png"].iter().collect();
		let converted_path = vfs.real_to_virtual(real_path.as_path()).unwrap();
		assert_eq!(converted_path, virtual_path);
	}

	#[test]
	fn cleans_path_string() {
		let mut correct_path = path::PathBuf::new();
		if cfg!(target_os = "windows") {
			correct_path.push("C:\\");
		} else {
			correct_path.push("/usr");
		}
		correct_path.push("some");
		correct_path.push("path");

		let tests = if cfg!(target_os = "windows") {
			vec![
				r#"C:/some/path"#,
				r#"C:\some\path"#,
				r#"C:\some\path\"#,
				r#"C:\some\path\\\\"#,
				r#"C:\some/path//"#,
			]
		} else {
			vec![
				r#"/usr/some/path"#,
				r#"/usr\some\path"#,
				r#"/usr\some\path\"#,
				r#"/usr\some\path\\\\"#,
				r#"/usr\some/path//"#,
			]
		};

		for test in tests {
			let mount_dir = MountDir {
				source: test.to_owned(),
				name: "name".to_owned(),
			};
			let mount: Mount = mount_dir.into();
			assert_eq!(mount.source, correct_path);
		}
	}
}
