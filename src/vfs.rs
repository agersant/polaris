use std::collections::HashMap;
use std::path::PathBuf;
use std::path::Path;

use error::*;

#[derive(Debug, Clone)]
pub struct VfsConfig {
	pub mount_points: HashMap<String, PathBuf>,
}

impl VfsConfig {
    pub fn new() -> VfsConfig {
        VfsConfig {
            mount_points: HashMap::new(),
        }
    }
}

pub struct Vfs {
    mount_points: HashMap<String, PathBuf>,
}

impl Vfs {
    pub fn new(config: VfsConfig) -> Vfs {
        Vfs { mount_points: config.mount_points }
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
fn test_virtual_to_real() {
    let mut config = VfsConfig::new();
    config.mount_points.insert("root".to_owned(), Path::new("test_dir").to_path_buf());
    let vfs = Vfs::new(config);

    let correct_path = Path::new("test_dir/somewhere/something.png");
    let found_path = vfs.virtual_to_real(Path::new("root/somewhere/something.png")).unwrap();
    assert!(found_path == correct_path);
}

#[test]
fn test_real_to_virtual() {
    let mut config = VfsConfig::new();
    config.mount_points.insert("root".to_owned(), Path::new("test_dir").to_path_buf());
    let vfs = Vfs::new(config);

    let correct_path = Path::new("root/somewhere/something.png");
    let found_path = vfs.real_to_virtual(Path::new("test_dir/somewhere/something.png")).unwrap();
    assert!(found_path == correct_path);
}
