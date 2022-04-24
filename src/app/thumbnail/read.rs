use anyhow::{bail, Result};
use image::DynamicImage;
use std::path::Path;

use crate::utils;
use crate::utils::AudioFormat;

pub fn read(image_path: &Path) -> Result<DynamicImage> {
	match utils::get_audio_format(image_path) {
		Some(AudioFormat::AIFF) => read_aiff(image_path),
		Some(AudioFormat::APE) => read_ape(image_path),
		Some(AudioFormat::FLAC) => read_flac(image_path),
		Some(AudioFormat::MP3) => read_mp3(image_path),
		Some(AudioFormat::MP4) => read_mp4(image_path),
		Some(AudioFormat::MPC) => read_ape(image_path),
		Some(AudioFormat::OGG) => read_vorbis(image_path),
		Some(AudioFormat::OPUS) => read_opus(image_path),
		Some(AudioFormat::WAVE) => read_wave(image_path),
		None => Ok(image::open(image_path)?),
	}
}

fn read_ape(_: &Path) -> Result<DynamicImage> {
	bail!("Embedded images are not supported in APE files");
}

fn read_flac(path: &Path) -> Result<DynamicImage> {
	let tag = metaflac::Tag::read_from_path(path)?;

	if let Some(p) = tag.pictures().next() {
		return Ok(image::load_from_memory(&p.data)?);
	}

	bail!(
		"Embedded flac artwork not found for file: {}",
		path.display()
	);
}

fn read_mp3(path: &Path) -> Result<DynamicImage> {
	let tag = id3::Tag::read_from_path(path)?;

	read_id3(path, &tag)
}

fn read_aiff(path: &Path) -> Result<DynamicImage> {
	let tag = id3::Tag::read_from_aiff_path(path)?;

	read_id3(path, &tag)
}

fn read_wave(path: &Path) -> Result<DynamicImage> {
	let tag = id3::Tag::read_from_wav_path(path)?;

	read_id3(path, &tag)
}

fn read_id3(path: &Path, tag: &id3::Tag) -> Result<DynamicImage> {
	if let Some(p) = tag.pictures().next() {
		return Ok(image::load_from_memory(&p.data)?);
	}

	bail!(
		"Embedded id3 artwork not found for file: {}",
		path.display()
	);
}

fn read_mp4(path: &Path) -> Result<DynamicImage> {
	let tag = mp4ameta::Tag::read_from_path(path)?;

	match tag.artwork().map(|d| d.data) {
		Some(v) => Ok(image::load_from_memory(v)?),
		_ => bail!(
			"Embedded mp4 artwork not found for file: {}",
			path.display()
		),
	}
}

fn read_vorbis(_: &Path) -> Result<DynamicImage> {
	bail!("Embedded images are not supported in Vorbis files");
}

fn read_opus(_: &Path) -> Result<DynamicImage> {
	bail!("Embedded images are not supported in Opus files");
}

#[test]
fn can_read_artwork_data() {
	let ext_img = image::open("test-data/artwork/Folder.png")
		.unwrap()
		.to_rgb8();
	let embedded_img = image::open("test-data/artwork/Embedded.png")
		.unwrap()
		.to_rgb8();

	let folder_img = read(Path::new("test-data/artwork/Folder.png"))
		.unwrap()
		.to_rgb8();
	assert_eq!(folder_img, ext_img);

	let aiff_img = read(Path::new("test-data/artwork/sample.aif"))
		.unwrap()
		.to_rgb8();
	assert_eq!(aiff_img, embedded_img);

	let ape_img = read(Path::new("test-data/artwork/sample.ape"))
		.map(|d| d.to_rgb8())
		.ok();
	assert_eq!(ape_img, None);

	let flac_img = read(Path::new("test-data/artwork/sample.flac"))
		.unwrap()
		.to_rgb8();
	assert_eq!(flac_img, embedded_img);

	let mp3_img = read(Path::new("test-data/artwork/sample.mp3"))
		.unwrap()
		.to_rgb8();
	assert_eq!(mp3_img, embedded_img);

	let m4a_img = read(Path::new("test-data/artwork/sample.m4a"))
		.unwrap()
		.to_rgb8();
	assert_eq!(m4a_img, embedded_img);

	let ogg_img = read(Path::new("test-data/artwork/sample.ogg"))
		.map(|d| d.to_rgb8())
		.ok();
	assert_eq!(ogg_img, None);

	let opus_img = read(Path::new("test-data/artwork/sample.opus"))
		.map(|d| d.to_rgb8())
		.ok();
	assert_eq!(opus_img, None);

	let wave_img = read(Path::new("test-data/artwork/sample.wav"))
		.unwrap()
		.to_rgb8();
	assert_eq!(wave_img, embedded_img);
}
