use std::collections::HashMap;
use std::path::PathBuf;
use std::path::Path;

use error::*;

pub struct Vfs {
	mount_points: HashMap<String, PathBuf>,
}

impl Vfs {
	pub fn new() -> Vfs {
		Vfs {
			mount_points: HashMap::new(),
		}
	}

	pub fn mount(&mut self, name: &str, real_path: &Path) -> Result<(), CollectionError>
	{
		let name = name.to_string(); 
		if self.mount_points.contains_key(&name)	{
			return Err(CollectionError::ConflictingMount(name));
		}
		self.mount_points.insert(name, real_path.to_path_buf());
		Ok(())
	}

	pub fn real_to_virtual(&self, real_path: &Path) -> Result<(), CollectionError> {
		Ok(())
	}

	pub fn virtual_to_real(&self, virtual_path: &Path) -> Result<(), CollectionError> {
		Ok(())
	}
}