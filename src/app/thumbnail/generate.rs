use anyhow::*;
use image::imageops::FilterType;
use image::{DynamicImage, GenericImage, GenericImageView, ImageBuffer};
use std::cmp;
use std::path::*;

use crate::app::thumbnail::{read, Options};

pub fn generate_thumbnail(image_path: &Path, thumbnailoptions: &Options) -> Result<DynamicImage> {
	let source_image = read(image_path)?;
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
