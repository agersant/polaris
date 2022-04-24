use anyhow::*;
use image::ImageOutputFormat;
use std::collections::hash_map::DefaultHasher;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use crate::app::thumbnail::*;

#[derive(Clone)]
pub struct Manager {
	thumbnails_dir_path: PathBuf,
}

impl Manager {
	pub fn new(thumbnails_dir_path: PathBuf) -> Self {
		Self {
			thumbnails_dir_path,
		}
	}

	pub fn get_thumbnail(&self, image_path: &Path, thumbnailoptions: &Options) -> Result<PathBuf> {
		match self.retrieve_thumbnail(image_path, thumbnailoptions) {
			Some(path) => Ok(path),
			None => self.create_thumbnail(image_path, thumbnailoptions),
		}
	}

	fn get_thumbnail_path(&self, image_path: &Path, thumbnailoptions: &Options) -> PathBuf {
		let hash = Manager::hash(image_path, thumbnailoptions);
		let mut thumbnail_path = self.thumbnails_dir_path.clone();
		thumbnail_path.push(format!("{}.jpg", hash));
		thumbnail_path
	}

	fn retrieve_thumbnail(&self, image_path: &Path, thumbnailoptions: &Options) -> Option<PathBuf> {
		let path = self.get_thumbnail_path(image_path, thumbnailoptions);
		if path.exists() {
			Some(path)
		} else {
			None
		}
	}

	fn create_thumbnail(&self, image_path: &Path, thumbnailoptions: &Options) -> Result<PathBuf> {
		let thumbnail = generate_thumbnail(image_path, thumbnailoptions)?;
		let quality = 80;

		fs::create_dir_all(&self.thumbnails_dir_path)?;
		let path = self.get_thumbnail_path(image_path, thumbnailoptions);
		let mut out_file = File::create(&path)?;
		thumbnail.write_to(&mut out_file, ImageOutputFormat::Jpeg(quality))?;
		Ok(path)
	}

	fn hash(path: &Path, thumbnailoptions: &Options) -> u64 {
		let mut hasher = DefaultHasher::new();
		path.hash(&mut hasher);
		thumbnailoptions.hash(&mut hasher);
		hasher.finish()
	}
}
