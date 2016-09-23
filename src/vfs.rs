use std::collections::HashMap;
use std::path::PathBuf;
use std::path::Path;

use error::*;

pub struct MountDir {
	pub name: String,
	pub path: PathBuf,
}

impl MountDir {
    pub fn new(name: String, path: PathBuf) -> MountDir {
        MountDir {
            name: name,
            path: path,
        }
    }
}

pub struct Vfs {
    mount_points: HashMap<String, PathBuf>,
}

impl Vfs {
    pub fn new() -> Vfs {
        let instance = Vfs { mount_points: HashMap::new() };
        instance
    }

    pub fn mount(&mut self, name: &str, real_path: &Path) -> Result<(), PError> {
        let name = name.to_string();
        if self.mount_points.contains_key(&name) {
            return Err(PError::ConflictingMount);
        }
        self.mount_points.insert(name, real_path.to_path_buf());
        Ok(())
    }

    pub fn real_to_virtual(&self, real_path: &Path) -> Result<PathBuf, PError> {
        for (name, target) in &self.mount_points {
            match real_path.strip_prefix(target) {
                Ok(p) => {
                    let mount_path = Path::new(&name);
                    return Ok(mount_path.join(p));
                }
                Err(_) => (),
            }
        }
        Err(PError::PathNotInVfs)
    }

    pub fn virtual_to_real(&self, virtual_path: &Path) -> Result<PathBuf, PError> {
        for (name, target) in &self.mount_points {
            let mount_path = Path::new(&name);
            match virtual_path.strip_prefix(mount_path) {
                Ok(p) => return Ok(target.join(p)),
                Err(_) => (),
            }
        }
        Err(PError::PathNotInVfs)
    }

    pub fn get_mount_points(&self) -> &HashMap<String, PathBuf> {
        return &self.mount_points;
    }
}

#[test]
fn test_mount() {
    let mut vfs = Vfs::new();
    assert!(vfs.mount("root", Path::new("test_dir")).is_ok());
    assert!(vfs.mount("root", Path::new("another_dir")).is_err());
}

#[test]
fn test_virtual_to_real() {
    let mut vfs = Vfs::new();
    assert!(vfs.mount("root", Path::new("test_dir")).is_ok());
    let correct_path = Path::new("test_dir/somewhere/something.png");
    let found_path = vfs.virtual_to_real(Path::new("root/somewhere/something.png")).unwrap();
    assert!(found_path == correct_path);
}

#[test]
fn test_real_to_virtual() {
    let mut vfs = Vfs::new();
    assert!(vfs.mount("root", Path::new("test_dir")).is_ok());
    let correct_path = Path::new("root/somewhere/something.png");
    let found_path = vfs.real_to_virtual(Path::new("test_dir/somewhere/something.png")).unwrap();
    assert!(found_path == correct_path);
}
