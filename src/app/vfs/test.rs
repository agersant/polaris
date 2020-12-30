use std::path::{Path, PathBuf};

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
