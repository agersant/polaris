use std::path::{Path, PathBuf};

use super::*;

#[test]
fn test_virtual_to_real() {
	let mut vfs = VFS::new();
	vfs.mount(Path::new("test_dir"), "root").unwrap();

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
	let mut vfs = VFS::new();
	vfs.mount(Path::new("test_dir"), "root").unwrap();
	let correct_path = Path::new("test_dir");
	let found_path = vfs.virtual_to_real(Path::new("root")).unwrap();
	assert!(found_path.to_str() == correct_path.to_str());
}

#[test]
fn test_real_to_virtual() {
	let mut vfs = VFS::new();
	vfs.mount(Path::new("test_dir"), "root").unwrap();

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
