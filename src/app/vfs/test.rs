use std::path::{Path, PathBuf};

use super::*;

#[test]
fn test_virtual_to_real() {
	let vfs = VFS::new(vec![Mount {
		name: "root".to_owned(),
		source: Path::new("test_dir").to_owned(),
	}]);

	let mut correct_path = PathBuf::new();
	correct_path.push("test_dir");
	correct_path.push("somewhere");
	correct_path.push("something.png");

	let mut virtual_path = PathBuf::new();
	virtual_path.push("root");
	virtual_path.push("somewhere");
	virtual_path.push("something.png");

	let found_path = vfs.virtual_to_real(virtual_path.as_path()).unwrap();
	assert!(found_path.to_str() == correct_path.to_str());
}

#[test]
fn test_virtual_to_real_no_trail() {
	let vfs = VFS::new(vec![Mount {
		name: "root".to_owned(),
		source: Path::new("test_dir").to_owned(),
	}]);
	let correct_path = Path::new("test_dir");
	let found_path = vfs.virtual_to_real(Path::new("root")).unwrap();
	assert!(found_path.to_str() == correct_path.to_str());
}

#[test]
fn test_real_to_virtual() {
	let vfs = VFS::new(vec![Mount {
		name: "root".to_owned(),
		source: Path::new("test_dir").to_owned(),
	}]);

	let mut correct_path = PathBuf::new();
	correct_path.push("root");
	correct_path.push("somewhere");
	correct_path.push("something.png");

	let mut real_path = PathBuf::new();
	real_path.push("test_dir");
	real_path.push("somewhere");
	real_path.push("something.png");

	let found_path = vfs.real_to_virtual(real_path.as_path()).unwrap();
	assert!(found_path == correct_path);
}

#[test]
fn test_clean_path_string() {
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
