use crate::directories::PolarisDirectories;
use anyhow::*;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Directories {}

impl Directories {
	pub fn get_pid_directory() -> Result<PathBuf> {
		let path = Path::new(option_env!("POLARIS_PID_DIR").unwrap_or("."));
		fs::create_dir_all(&path)?;
		Ok(path.to_owned())
	}
}

impl PolarisDirectories for Directories {
	fn get_static_directory() -> Result<PathBuf> {
		let path = Path::new(option_env!("POLARIS_STATIC_DIR").unwrap_or("."));
		fs::create_dir_all(&path)?;
		Ok(path.to_owned())
	}

	fn get_db_directory() -> Result<PathBuf> {
		let path = Path::new(option_env!("POLARIS_DB_DIR").unwrap_or("."));
		fs::create_dir_all(&path)?;
		Ok(path.to_owned())
	}

	fn get_log_directory() -> Result<PathBuf> {
		let path = Path::new(option_env!("POLARIS_LOG_DIR").unwrap_or("."));
		fs::create_dir_all(&path)?;
		Ok(path.to_owned())
	}

	fn get_thumbnail_directory() -> Result<PathBuf> {
		let mut path = Path::new(option_env!("POLARIS_CACHE_DIR").unwrap_or(".")).to_owned();
		path.push("thumbnails");
		fs::create_dir_all(&path)?;
		Ok(path.to_owned())
	}
}
