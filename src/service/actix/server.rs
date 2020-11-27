use anyhow::*;
use std::path::Path;

use crate::db::DB;
use crate::index::Index;
use crate::thumbnails::ThumbnailsManager;

pub fn run(
	port: u16,
	auth_secret: &[u8],
	api_url: &str,
	web_url: &str,
	web_dir_path: &Path,
	swagger_url: &str,
	swagger_dir_path: &Path,
	db: DB,
	command_sender: Index,
	thumbnails_manager: ThumbnailsManager,
) -> Result<()> {
	Ok(())
}
