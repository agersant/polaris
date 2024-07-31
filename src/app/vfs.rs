use core::ops::Deref;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::{Acquire, QueryBuilder, Sqlite};
use std::path::{self, Path, PathBuf};

use crate::db::{self, DB};

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("The following virtual path could not be mapped to a real path: `{0}`")]
	CouldNotMapToRealPath(PathBuf),
	#[error(transparent)]
	DatabaseConnection(#[from] db::Error),
	#[error(transparent)]
	Database(#[from] sqlx::Error),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
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

	pub fn virtual_to_real<P: AsRef<Path>>(&self, virtual_path: P) -> Result<PathBuf, Error> {
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
		Err(Error::CouldNotMapToRealPath(virtual_path.as_ref().into()))
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

	pub async fn get_vfs(&self) -> Result<VFS, Error> {
		let mount_dirs = self.mount_dirs().await?;
		let mounts = mount_dirs.into_iter().map(|p| p.into()).collect();
		Ok(VFS::new(mounts))
	}

	pub async fn mount_dirs(&self) -> Result<Vec<MountDir>, Error> {
		Ok(
			sqlx::query_as!(MountDir, "SELECT source, name FROM mount_points")
				.fetch_all(self.db.connect().await?.as_mut())
				.await?,
		)
	}

	pub async fn set_mount_dirs(&self, mount_dirs: &[MountDir]) -> Result<(), Error> {
		let mut connection = self.db.connect().await?;

		connection.begin().await?;

		sqlx::query!("DELETE FROM mount_points")
			.execute(connection.as_mut())
			.await?;

		if !mount_dirs.is_empty() {
			QueryBuilder::<Sqlite>::new("INSERT INTO mount_points(source, name) ")
				.push_values(mount_dirs, |mut b, dir| {
					b.push_bind(&dir.source).push_bind(&dir.name);
				})
				.build()
				.execute(connection.as_mut())
				.await?;
		}

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
