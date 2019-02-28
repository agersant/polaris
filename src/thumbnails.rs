use image;
use image::DynamicImage;
use image::FilterType;
use image::GenericImage;
use image::GenericImageView;
use image::ImageBuffer;
use image::ImageOutputFormat;
use std::cmp;
use std::collections::hash_map::DefaultHasher;
use std::fs::{DirBuilder, File};
use std::hash::{Hash, Hasher};
use std::path::*;

use crate::errors::*;
use crate::utils;

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
	let largest_dimension = cmp::max(source_width, source_height);
	let out_dimension = cmp::min(max_dimension, largest_dimension);

	let hash = hash(real_path, out_dimension);
	out_path.push(format!("{}.jpg", hash.to_string()));

	if !out_path.exists() {
		let quality = 80;
		let source_aspect_ratio: f32 = source_width as f32 / source_height as f32;

		let mut final_image;
		if source_aspect_ratio < 0.8 || source_aspect_ratio > 1.2 {
			let scaled_image =
				source_image.resize(out_dimension, out_dimension, FilterType::Lanczos3);
			let (scaled_width, scaled_height) = scaled_image.dimensions();
			let background = image::Rgb {
				data: [255 as u8, 255 as u8, 255 as u8],
			};
			final_image = DynamicImage::ImageRgb8(ImageBuffer::from_pixel(
				out_dimension,
				out_dimension,
				background,
			));
			final_image.copy_from(
				&scaled_image,
				(out_dimension - scaled_width) / 2,
				(out_dimension - scaled_height) / 2,
			);
		} else {
			final_image =
				source_image.resize_exact(out_dimension, out_dimension, FilterType::Lanczos3);
		};

		let mut out_file = File::create(&out_path)?;
		final_image.write_to(&mut out_file, ImageOutputFormat::JPEG(quality))?;
	}

	Ok(out_path)
}
