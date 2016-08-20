use std::collections::HashMap;
use std::path::PathBuf;
use std::path::Path;

use error::*;

pub struct Vfs {
	mount_points: HashMap<String, PathBuf>,
}

impl Vfs {
	pub fn new() -> Vfs {
		let mut instance = Vfs {
			mount_points: HashMap::new(),
		};
		instance.mount( "root", Path::new("samplemusic") );
		instance
	}

	pub fn mount(&mut self, name: &str, real_path: &Path) -> Result<(), CollectionError>
	{
		let name = name.to_string(); 
		if self.mount_points.contains_key(&name)	{
			return Err(CollectionError::ConflictingMount);
		}
		self.mount_points.insert(name, real_path.to_path_buf());
		Ok(())
	}

	pub fn real_to_virtual(&self, real_path: &Path) -> Result<PathBuf, CollectionError> {
		for (name, target) in &self.mount_points {
			match real_path.strip_prefix(target) {
				Ok(p) => {
					let mount_path = Path::new(&name);
					return Ok(mount_path.join(p));
				},
				Err(_) => (),
			}
		}
		Err(CollectionError::PathNotInVfs)
	}

	pub fn virtual_to_real(&self, virtual_path: &Path) -> Result<PathBuf, CollectionError> {
		for (name, target) in &self.mount_points {
			let mount_path = Path::new(&name);
			match virtual_path.strip_prefix(mount_path) {
				Ok(p) => return Ok(target.join(p)),
				Err(_) => (),
			}
		}
		Err(CollectionError::PathNotInVfs)
	}
}