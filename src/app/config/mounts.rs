use std::{
	ops::Deref,
	path::{Path, PathBuf},
};

use regex::Regex;

use crate::app::Error;

use super::raw;
use super::Config;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MountDir {
	pub source: PathBuf,
	pub name: String,
}

impl TryFrom<raw::MountDir> for MountDir {
	type Error = Error;

	fn try_from(mount_dir: raw::MountDir) -> Result<Self, Self::Error> {
		// TODO validation
		Ok(Self {
			source: sanitize_path(&mount_dir.source),
			name: mount_dir.name,
		})
	}
}

impl Config {
	pub fn set_mounts(&mut self, mount_dirs: Vec<raw::MountDir>) {
		self.mount_dirs = mount_dirs
			.into_iter()
			.filter_map(|m| m.try_into().ok())
			.collect();
	}

	pub fn resolve_virtual_path<P: AsRef<Path>>(&self, virtual_path: P) -> Result<PathBuf, Error> {
		for mount in &self.mount_dirs {
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
}

fn sanitize_path(source: &str) -> PathBuf {
	let separator_regex = Regex::new(r"\\|/").unwrap();
	let mut correct_separator = String::new();
	correct_separator.push(std::path::MAIN_SEPARATOR);
	let path_string = separator_regex.replace_all(source, correct_separator.as_str());
	PathBuf::from(path_string.deref())
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn can_resolve_virtual_paths() {
		let raw_config = raw::Config {
			mount_dirs: vec![raw::MountDir {
				name: "root".to_owned(),
				source: "test_dir".to_owned(),
			}],
			..Default::default()
		};

		let config: Config = raw_config.try_into().unwrap();

		let test_cases = vec![
			(vec!["root"], vec!["test_dir"]),
			(
				vec!["root", "somewhere", "something.png"],
				vec!["test_dir", "somewhere", "something.png"],
			),
		];

		for (r#virtual, real) in test_cases {
			let real_path: PathBuf = real.iter().collect();
			let virtual_path: PathBuf = r#virtual.iter().collect();
			let converted_path = config.resolve_virtual_path(&virtual_path).unwrap();
			assert_eq!(converted_path, real_path);
		}
	}

	#[test]
	fn sanitizes_paths() {
		let mut correct_path = PathBuf::new();
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
			let raw_config = raw::Config {
				mount_dirs: vec![raw::MountDir {
					name: "root".to_owned(),
					source: test.to_owned(),
				}],
				..Default::default()
			};
			let config: Config = raw_config.try_into().unwrap();
			let converted_path = config.resolve_virtual_path(&PathBuf::from("root")).unwrap();
			assert_eq!(converted_path, correct_path);
		}
	}
}
