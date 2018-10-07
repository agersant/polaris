use image;
use image::FilterType;
use image::GenericImage;
use image::GenericImageView;
use image::ImageBuffer;
use std::cmp;
use std::collections::hash_map::DefaultHasher;
use std::fs::{DirBuilder};
use std::hash::{Hash, Hasher};
use std::path::*;

use errors::*;
use utils;

const THUMBNAILS_PATH: &str = "thumbnails";

fn hash(path: &Path, dimension: u32) -> u64 {
	let path_string = path.to_string_lossy();
	let hash_input = format!("{}:{}", path_string, dimension.to_string());
	let mut hasher = DefaultHasher::new();
	hash_input.hash(&mut hasher);
	hasher.finish()
}

pub fn get_thumbnail(real_path: &Path, max_dimension: u32) -> Result<PathBuf> {
	let mut out_path = utils::get_data_root()?;
	out_path.push(THUMBNAILS_PATH);

	let mut dir_builder = DirBuilder::new();
	dir_builder.recursive(true);
	dir_builder.create(out_path.as_path())?;

	let source_image = image::open(real_path)?;
	let (source_width, source_height) = source_image.dimensions();
	let cropped_dimension = cmp::max(source_width, source_height);
	let out_dimension = cmp::min(max_dimension, cropped_dimension);

	let hash = hash(real_path, out_dimension);
	out_path.push(format!("{}.png", hash.to_string()));

	if !out_path.exists() {
		let source_aspect_ratio: f32 = source_width as f32 / source_height as f32;
		if source_aspect_ratio < 0.8 || source_aspect_ratio > 1.2 {
			let scaled_image =
				source_image.resize(out_dimension, out_dimension, FilterType::Lanczos3);
			let (scaled_width, scaled_height) = scaled_image.dimensions();
			let mut final_image = ImageBuffer::new(out_dimension, out_dimension);
			final_image.copy_from(
				&scaled_image,
				(out_dimension - scaled_width) / 2,
				(out_dimension - scaled_height) / 2,
			);
			final_image.save(&out_path)?;
		} else {
			let final_image =
				source_image.resize_exact(max_dimension, out_dimension, FilterType::Lanczos3);
			final_image.save(&out_path)?;
		};
	}

	Ok(out_path)
}
