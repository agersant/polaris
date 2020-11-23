use anyhow::*;
use image::DynamicImage;
use std::path::Path;

use crate::utils;
use crate::utils::AudioFormat;

pub fn read_artwork(image_path: &Path) -> Result<DynamicImage> {
	match utils::get_audio_format(image_path) {
		Some(AudioFormat::APE) => read_ape_artwork(image_path),
		Some(AudioFormat::FLAC) => read_flac_artwork(image_path),
		Some(AudioFormat::MP3) => read_id3_artwork(image_path),
		Some(AudioFormat::MP4) => read_mp4_artwork(image_path),
		Some(AudioFormat::MPC) => read_ape_artwork(image_path),
		Some(AudioFormat::OGG) => read_vorbis_artwork(image_path),
		Some(AudioFormat::OPUS) => read_opus_artwork(image_path),
		None => Ok(image::open(image_path)?),
	}
}

fn read_ape_artwork(_: &Path) -> Result<DynamicImage> {
	Err(crate::Error::msg("Embedded ape artworks not yet supported"))
}

fn read_flac_artwork(_: &Path) -> Result<DynamicImage> {
	Err(crate::Error::msg(
		"Embedded flac artworks are not yet supported",
	))
}

fn read_id3_artwork(path: &Path) -> Result<DynamicImage> {
	let tag = id3::Tag::read_from_path(path)?;

	if let Some(p) = tag.pictures().next() {
		return Ok(image::load_from_memory(&p.data)?);
	}

	Err(crate::Error::msg(format!(
		"Embedded id3 artwork not found for file: {}",
		path.display()
	)))
}

fn read_mp4_artwork(path: &Path) -> Result<DynamicImage> {
	let tag = mp4ameta::Tag::read_from_path(path)?;

	match tag.artwork() {
		Some(mp4ameta::Data::Jpeg(v)) => Ok(image::load_from_memory(v)?),
		Some(mp4ameta::Data::Png(v)) => Ok(image::load_from_memory(v)?),
		_ => Err(crate::Error::msg(format!(
			"Embedded mp4 artwork not found for file: {}",
			path.display()
		))),
	}
}

fn read_vorbis_artwork(_: &Path) -> Result<DynamicImage> {
	Err(crate::Error::msg(
		"Embedded vorbis artworks are not yet supported",
	))
}

fn read_opus_artwork(_: &Path) -> Result<DynamicImage> {
	Err(crate::Error::msg(
		"Embedded opus artworks are not yet supported",
	))
}
