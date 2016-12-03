use std::collections::HashMap;
use std::path::PathBuf;
use std::path::Path;

use errors::*;

#[derive(Debug, Clone)]
pub struct VfsConfig {
    pub mount_points: HashMap<String, PathBuf>,
}

impl VfsConfig {
    pub fn new() -> VfsConfig {
        VfsConfig { mount_points: HashMap::new() }
    }
}

pub struct Vfs {
    mount_points: HashMap<String, PathBuf>,
}

impl Vfs {
    pub fn new(config: VfsConfig) -> Vfs {
        Vfs { mount_points: config.mount_points }
    }

    pub fn real_to_virtual(&self, real_path: &Path) -> Result<PathBuf> {
        for (name, target) in &self.mount_points {
            match real_path.strip_prefix(target) {
                Ok(p) => {
                    let mount_path = Path::new(&name);
                    return Ok(mount_path.join(p));
                }
                Err(_) => (),
            }
        }
        bail!("Real path has no match in VFS")
    }

    pub fn virtual_to_real(&self, virtual_path: &Path) -> Result<PathBuf> {
        for (name, target) in &self.mount_points {
            let mount_path = Path::new(&name);
            match virtual_path.strip_prefix(mount_path) {
                Ok(p) => {
                    return if p.components().count() == 0 {
                        Ok(target.clone())
                    } else {
                        Ok(target.join(p))
                    }
                }
                Err(_) => (),
            }
        }
        bail!("Virtual path has no match in VFS")
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
    let mut config = VfsConfig::new();
    config.mount_points.insert("root".to_owned(), Path::new("test_dir").to_path_buf());
    let vfs = Vfs::new(config);
    let correct_path = Path::new("test_dir");
    let found_path = vfs.virtual_to_real(Path::new("root")).unwrap();
    assert!(found_path.to_str() == correct_path.to_str());
}

#[test]
fn test_real_to_virtual() {
    let mut config = VfsConfig::new();
    config.mount_points.insert("root".to_owned(), Path::new("test_dir").to_path_buf());
    let vfs = Vfs::new(config);

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
