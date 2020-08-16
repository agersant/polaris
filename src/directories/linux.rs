use crate::directories::PolarisDirectories;
use anyhow::*;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Directories {}

impl Directories {
	pub fn get_pid_directory() -> Result<PathBuf> {
		let path = Path::new("/run/polaris");
		fs::create_dir_all(&path)?;
		Ok(path.to_owned())
	}
}

impl PolarisDirectories for Directories {
	fn get_log_directory() -> Result<PathBuf> {
		let path = Path::new("/var/log/polaris");
		fs::create_dir_all(&path)?;
		Ok(path.to_owned())
	}

	fn get_db_directory() -> Result<PathBuf> {
		let path = Path::new("/var/lib/polaris");
		fs::create_dir_all(&path)?;
		Ok(path.to_owned())
	}

	fn get_thumbnail_directory() -> Result<PathBuf> {
		let path = Path::new("/var/cache/polaris/thumbnails");
		fs::create_dir_all(&path)?;
		Ok(path.to_owned())
	}

	fn get_static_directory() -> Result<PathBuf> {
		let path = Path::new("/usr/share/polaris");
		fs::create_dir_all(&path)?;
		Ok(path.to_owned())
	}
}
