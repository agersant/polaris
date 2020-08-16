use anyhow::*;
use std::path::PathBuf;

#[cfg(target_family = "windows")]
mod windows;

#[cfg(target_family = "windows")]
pub use self::windows::Directories;

#[cfg(not(target_family = "windows"))]
mod linux;

#[cfg(not(target_family = "windows"))]
pub use self::linux::Directories;

pub trait PolarisDirectories {
	fn get_web_directory() -> Result<PathBuf>;
	fn get_swagger_directory() -> Result<PathBuf>;
	fn get_db_directory() -> Result<PathBuf>;
	fn get_log_directory() -> Result<PathBuf>;
	fn get_thumbnail_directory() -> Result<PathBuf>;
}
