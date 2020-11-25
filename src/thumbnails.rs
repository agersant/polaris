use anyhow::*;
use image::imageops::FilterType;
use image::{DynamicImage, GenericImage, GenericImageView, ImageBuffer, ImageOutputFormat};
use std::cmp;
use std::collections::hash_map::DefaultHasher;
use std::fs::{DirBuilder, File};
use std::hash::{Hash, Hasher};
use std::path::*;

use crate::artwork;

pub struct ThumbnailsManager {
	thumbnails_path: PathBuf,
}

impl ThumbnailsManager {
	pub fn new(thumbnails_path: &Path) -> ThumbnailsManager {
		ThumbnailsManager {
			thumbnails_path: thumbnails_path.to_owned(),
		}
	}

	pub fn get_thumbnail(
		&self,
		image_path: &Path,
		thumbnailoptions: &ThumbnailOptions,
	) -> Result<PathBuf> {
		match self.retrieve_thumbnail(image_path, thumbnailoptions) {
			Some(path) => Ok(path),
			None => self.create_thumbnail(image_path, thumbnailoptions),
		}
	}

	fn create_thumbnails_directory(&self) -> Result<()> {
		let mut dir_builder = DirBuilder::new();
		dir_builder.recursive(true);
		dir_builder.create(self.thumbnails_path.as_path())?;
		Ok(())
	}

	fn get_thumbnail_path(
		&self,
		image_path: &Path,
		thumbnailoptions: &ThumbnailOptions,
	) -> PathBuf {
		let hash = hash(image_path, thumbnailoptions);
		let mut thumbnail_path = self.thumbnails_path.clone();
		thumbnail_path.push(format!("{}.jpg", hash.to_string()));
		thumbnail_path
	}

	fn retrieve_thumbnail(
		&self,
		image_path: &Path,
		thumbnailoptions: &ThumbnailOptions,
	) -> Option<PathBuf> {
		let path = self.get_thumbnail_path(image_path, thumbnailoptions);
		if path.exists() {
			Some(path)
		} else {
			None
		}
	}

	fn create_thumbnail(
		&self,
		image_path: &Path,
		thumbnailoptions: &ThumbnailOptions,
	) -> Result<PathBuf> {
		let thumbnail = generate_thumbnail(image_path, thumbnailoptions)?;
		let quality = 80;

		self.create_thumbnails_directory()?;
		let path = self.get_thumbnail_path(image_path, thumbnailoptions);
		let mut out_file = File::create(&path)?;
		thumbnail.write_to(&mut out_file, ImageOutputFormat::Jpeg(quality))?;
		Ok(path)
	}
}

fn hash(path: &Path, thumbnailoptions: &ThumbnailOptions) -> u64 {
	let mut hasher = DefaultHasher::new();
	path.hash(&mut hasher);
	thumbnailoptions.hash(&mut hasher);
	hasher.finish()
}

fn generate_thumbnail(
	image_path: &Path,
	thumbnailoptions: &ThumbnailOptions,
) -> Result<DynamicImage> {
	let source_image = artwork::read(image_path)?;
	let (source_width, source_height) = source_image.dimensions();
	let largest_dimension = cmp::max(source_width, source_height);
	let out_dimension = cmp::min(thumbnailoptions.max_dimension, largest_dimension);

	let source_aspect_ratio: f32 = source_width as f32 / source_height as f32;
	let is_almost_square = source_aspect_ratio > 0.8 && source_aspect_ratio < 1.2;

	let mut final_image;
	if is_almost_square && thumbnailoptions.resize_if_almost_square {
		final_image = source_image.resize_exact(out_dimension, out_dimension, FilterType::Lanczos3);
	} else if thumbnailoptions.pad_to_square {
		let scaled_image = source_image.resize(out_dimension, out_dimension, FilterType::Lanczos3);
		let (scaled_width, scaled_height) = scaled_image.dimensions();
		let background = image::Rgb([255, 255 as u8, 255 as u8]);
		final_image = DynamicImage::ImageRgb8(ImageBuffer::from_pixel(
			out_dimension,
			out_dimension,
			background,
		));
		final_image.copy_from(
			&scaled_image,
			(out_dimension - scaled_width) / 2,
			(out_dimension - scaled_height) / 2,
		)?;
	} else {
		final_image = source_image.resize(out_dimension, out_dimension, FilterType::Lanczos3);
	}

	Ok(final_image)
}

#[derive(Debug, Hash)]
pub struct ThumbnailOptions {
	pub max_dimension: u32,
	pub resize_if_almost_square: bool,
	pub pad_to_square: bool,
}

impl Default for ThumbnailOptions {
	fn default() -> ThumbnailOptions {
		ThumbnailOptions {
			max_dimension: 400,
			resize_if_almost_square: true,
			pad_to_square: true,
		}
	}
}
