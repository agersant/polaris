use anyhow::*;
use ape;
use id3;
use lewton::inside_ogg::OggStreamReader;
use log::error;
use metaflac;
use mp3_duration;
use mp4ameta;
use opus_headers;
use regex::Regex;
use std::fs;
use std::path::Path;

use crate::utils;
use crate::utils::AudioFormat;

#[derive(Debug, Clone, PartialEq)]
pub struct SongTags {
	pub disc_number: Option<u32>,
	pub track_number: Option<u32>,
	pub title: Option<String>,
	pub duration: Option<u32>,
	pub artist: Option<String>,
	pub album_artist: Option<String>,
	pub album: Option<String>,
	pub year: Option<i32>,
	pub artwork: bool,
}

#[cfg_attr(feature = "profile-index", flame)]
pub fn read(path: &Path) -> Option<SongTags> {
	let data = match utils::get_audio_format(path) {
		Some(AudioFormat::APE) => Some(read_ape(path)),
		Some(AudioFormat::FLAC) => Some(read_flac(path)),
		Some(AudioFormat::MP3) => Some(read_id3(path)),
		Some(AudioFormat::MP4) => Some(read_mp4(path)),
		Some(AudioFormat::MPC) => Some(read_ape(path)),
		Some(AudioFormat::OGG) => Some(read_vorbis(path)),
		Some(AudioFormat::OPUS) => Some(read_opus(path)),
		None => None,
	};
	match data {
		Some(Ok(d)) => Some(d),
		Some(Err(e)) => {
			error!("Error while reading file metadata for '{:?}': {}", path, e);
			None
		}
		None => None,
	}
}

#[cfg_attr(feature = "profile-index", flame)]
fn read_id3(path: &Path) -> Result<SongTags> {
	let tag = {
		#[cfg(feature = "profile-index")]
		let _guard = flame::start_guard("id3_tag_read");
		match id3::Tag::read_from_path(&path) {
			Ok(t) => Ok(t),
			Err(e) => {
				if let Some(t) = e.partial_tag {
					Ok(t)
				} else {
					Err(e)
				}
			}
		}?
	};
	let duration = {
		#[cfg(feature = "profile-index")]
		let _guard = flame::start_guard("mp3_duration");
		mp3_duration::from_path(&path)
			.map(|d| d.as_secs() as u32)
			.ok()
	};

	let artist = tag.artist().map(|s| s.to_string());
	let album_artist = tag.album_artist().map(|s| s.to_string());
	let album = tag.album().map(|s| s.to_string());
	let title = tag.title().map(|s| s.to_string());
	let disc_number = tag.disc();
	let track_number = tag.track();
	let year = tag
		.year()
		.map(|y| y as i32)
		.or_else(|| tag.date_released().and_then(|d| Some(d.year)))
		.or_else(|| tag.date_recorded().and_then(|d| Some(d.year)));
	let artwork = tag.pictures().count() > 0;

	Ok(SongTags {
		artist,
		album_artist,
		album,
		title,
		duration,
		disc_number,
		track_number,
		year,
		artwork,
	})
}

fn read_ape_string(item: &ape::Item) -> Option<String> {
	match item.value {
		ape::ItemValue::Text(ref s) => Some(s.clone()),
		_ => None,
	}
}

fn read_ape_i32(item: &ape::Item) -> Option<i32> {
	match item.value {
		ape::ItemValue::Text(ref s) => s.parse::<i32>().ok(),
		_ => None,
	}
}

fn read_ape_x_of_y(item: &ape::Item) -> Option<u32> {
	match item.value {
		ape::ItemValue::Text(ref s) => {
			let format = Regex::new(r#"^\d+"#).unwrap();
			if let Some(m) = format.find(s) {
				s[m.start()..m.end()].parse().ok()
			} else {
				None
			}
		}
		_ => None,
	}
}

#[cfg_attr(feature = "profile-index", flame)]
fn read_ape(path: &Path) -> Result<SongTags> {
	let tag = ape::read(path)?;
	let artist = tag.item("Artist").and_then(read_ape_string);
	let album = tag.item("Album").and_then(read_ape_string);
	let album_artist = tag.item("Album artist").and_then(read_ape_string);
	let title = tag.item("Title").and_then(read_ape_string);
	let year = tag.item("Year").and_then(read_ape_i32);
	let disc_number = tag.item("Disc").and_then(read_ape_x_of_y);
	let track_number = tag.item("Track").and_then(read_ape_x_of_y);
	Ok(SongTags {
		artist,
		album_artist,
		album,
		title,
		duration: None,
		disc_number,
		track_number,
		year,
		artwork: false,
	})
}

#[cfg_attr(feature = "profile-index", flame)]
fn read_vorbis(path: &Path) -> Result<SongTags> {
	let file = fs::File::open(path)?;
	let source = OggStreamReader::new(file)?;

	let mut tags = SongTags {
		artist: None,
		album_artist: None,
		album: None,
		title: None,
		duration: None,
		disc_number: None,
		track_number: None,
		year: None,
		artwork: false,
	};

	for (key, value) in source.comment_hdr.comment_list {
		utils::match_ignore_case! {
			match key {
				"TITLE" => tags.title = Some(value),
				"ALBUM" => tags.album = Some(value),
				"ARTIST" => tags.artist = Some(value),
				"ALBUMARTIST" => tags.album_artist = Some(value),
				"TRACKNUMBER" => tags.track_number = value.parse::<u32>().ok(),
				"DISCNUMBER" => tags.disc_number = value.parse::<u32>().ok(),
				"DATE" => tags.year = value.parse::<i32>().ok(),
				_ => (),
			}
		}
	}

	Ok(tags)
}

#[cfg_attr(feature = "profile-index", flame)]
fn read_opus(path: &Path) -> Result<SongTags> {
	let headers = opus_headers::parse_from_path(path)?;

	let mut tags = SongTags {
		artist: None,
		album_artist: None,
		album: None,
		title: None,
		duration: None,
		disc_number: None,
		track_number: None,
		year: None,
		artwork: false,
	};

	for (key, value) in headers.comments.user_comments {
		utils::match_ignore_case! {
			match key {
				"TITLE" => tags.title = Some(value),
				"ALBUM" => tags.album = Some(value),
				"ARTIST" => tags.artist = Some(value),
				"ALBUMARTIST" => tags.album_artist = Some(value),
				"TRACKNUMBER" => tags.track_number = value.parse::<u32>().ok(),
				"DISCNUMBER" => tags.disc_number = value.parse::<u32>().ok(),
				"DATE" => tags.year = value.parse::<i32>().ok(),
				_ => (),
			}
		}
	}

	Ok(tags)
}

#[cfg_attr(feature = "profile-index", flame)]
fn read_flac(path: &Path) -> Result<SongTags> {
	let tag = metaflac::Tag::read_from_path(path)?;
	let vorbis = tag
		.vorbis_comments()
		.ok_or(anyhow!("Missing Vorbis comments"))?;
	let disc_number = vorbis
		.get("DISCNUMBER")
		.and_then(|d| d[0].parse::<u32>().ok());
	let year = vorbis.get("DATE").and_then(|d| d[0].parse::<i32>().ok());
	let mut streaminfo = tag.get_blocks(metaflac::BlockType::StreamInfo);
	let duration = match streaminfo.next() {
		Some(&metaflac::Block::StreamInfo(ref s)) => {
			Some((s.total_samples as u32 / s.sample_rate) as u32)
		}
		_ => None,
	};
	let artwork = tag.pictures().count() > 0;

	Ok(SongTags {
		artist: vorbis.artist().map(|v| v[0].clone()),
		album_artist: vorbis.album_artist().map(|v| v[0].clone()),
		album: vorbis.album().map(|v| v[0].clone()),
		title: vorbis.title().map(|v| v[0].clone()),
		duration,
		disc_number,
		track_number: vorbis.track(),
		year,
		artwork,
	})
}

#[cfg_attr(feature = "profile-index", flame)]
fn read_mp4(path: &Path) -> Result<SongTags> {
	let mut tag = mp4ameta::Tag::read_from_path(path)?;

	Ok(SongTags {
		artist: tag.take_artist(),
		album_artist: tag.take_album_artist(),
		album: tag.take_album(),
		title: tag.take_title(),
		duration: tag.duration().map(|v| v as u32),
		disc_number: tag.disc_number().map(|d| d as u32),
		track_number: tag.track_number().map(|d| d as u32),
		year: tag.year().and_then(|v| v.parse::<i32>().ok()),
		artwork: tag.artwork().is_some(),
	})
}

#[test]
fn test_read_metadata() {
	let sample_tags = SongTags {
		disc_number: Some(3),
		track_number: Some(1),
		title: Some("TEST TITLE".into()),
		artist: Some("TEST ARTIST".into()),
		album_artist: Some("TEST ALBUM ARTIST".into()),
		album: Some("TEST ALBUM".into()),
		duration: None,
		year: Some(2016),
		artwork: false,
	};
	let flac_sample_tag = SongTags {
		duration: Some(0),
		..sample_tags.clone()
	};
	let mp3_sample_tag = SongTags {
		duration: Some(0),
		..sample_tags.clone()
	};
	let m4a_sample_tag = SongTags {
		duration: Some(0),
		..sample_tags.clone()
	};
	assert_eq!(
		read(Path::new("test-data/formats/sample.mp3")).unwrap(),
		mp3_sample_tag
	);
	assert_eq!(
		read(Path::new("test-data/formats/sample.ogg")).unwrap(),
		sample_tags
	);
	assert_eq!(
		read(Path::new("test-data/formats/sample.flac")).unwrap(),
		flac_sample_tag
	);
	assert_eq!(
		read(Path::new("test-data/formats/sample.m4a")).unwrap(),
		m4a_sample_tag
	);
	assert_eq!(
		read(Path::new("test-data/formats/sample.opus")).unwrap(),
		sample_tags
	);
	assert_eq!(
		read(Path::new("test-data/formats/sample.ape")).unwrap(),
		sample_tags
	);
}
