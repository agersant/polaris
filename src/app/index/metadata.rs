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

/// For reference: https://wiki.hydrogenaud.io/index.php?title=Tag_Mapping
#[derive(Debug, Clone, PartialEq)]
pub struct SongTags {
	pub artist: Option<String>,
	pub album_artist: Option<String>,
	pub composer: Option<String>,
	pub lyricist: Option<String>,
	pub album: Option<String>,
	pub title: Option<String>,
	pub disc_number: Option<u32>,
	pub track_number: Option<u32>,
	pub genre: Option<String>,
	pub year: Option<i32>,
	pub duration: Option<u32>,
	pub has_artwork: bool,
}

impl From<id3::Tag> for SongTags {
	fn from(tag: id3::Tag) -> Self {
		let artist = tag.artist().map(|s| s.to_string());
		let album_artist = tag.album_artist().map(|s| s.to_string());
		let composer = tag.get("TCOM").map(|s| s.to_string());
		let lyricist = tag.get("TEXT").map(|s| s.to_string());
		let album = tag.album().map(|s| s.to_string());
		let title = tag.title().map(|s| s.to_string());
		let disc_number = tag.disc();
		let track_number = tag.track();
		let genre = tag.genre().map(|s| s.to_string());
		let year = tag
			.year()
			.map(|y| y as i32)
			.or_else(|| tag.date_released().map(|d| d.year))
			.or_else(|| tag.date_recorded().map(|d| d.year));
		let duration = tag.duration();
		let has_artwork = tag.pictures().count() > 0;

		SongTags {
			artist,
			album_artist,
			composer,
			lyricist,
			album,
			title,
			duration,
			disc_number,
			track_number,
			genre,
			year,
			has_artwork,
		}
	}
}

pub fn read(path: &Path) -> Option<SongTags> {
	let data = match utils::get_audio_format(path) {
		Some(AudioFormat::AIFF) => Some(read_aiff(path)),
		Some(AudioFormat::APE) => Some(read_ape(path)),
		Some(AudioFormat::FLAC) => Some(read_flac(path)),
		Some(AudioFormat::MP3) => Some(read_mp3(path)),
		Some(AudioFormat::MP4) => Some(read_mp4(path)),
		Some(AudioFormat::MPC) => Some(read_ape(path)),
		Some(AudioFormat::OGG) => Some(read_vorbis(path)),
		Some(AudioFormat::OPUS) => Some(read_opus(path)),
		Some(AudioFormat::WAVE) => Some(read_wave(path)),
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

fn read_mp3(path: &Path) -> Result<SongTags> {
	let tag = id3::Tag::read_from_path(&path).or_else(|error| {
		if let Some(tag) = error.partial_tag {
			Ok(tag)
		} else {
			Err(error)
		}
	})?;

	let duration = {
		mp3_duration::from_path(&path)
			.map(|d| d.as_secs() as u32)
			.ok()
	};

	let mut song_tags: SongTags = tag.into();
	song_tags.duration = duration; // Use duration from mp3_duration instead of from tags.
	Ok(song_tags)
}

fn read_aiff(path: &Path) -> Result<SongTags> {
	let tag = id3::Tag::read_from_aiff(&path).or_else(|error| {
		if let Some(tag) = error.partial_tag {
			Ok(tag)
		} else {
			Err(error)
		}
	})?;
	Ok(tag.into())
}

fn read_wave(path: &Path) -> Result<SongTags> {
	let tag = id3::Tag::read_from_wav(&path).or_else(|error| {
		if let Some(tag) = error.partial_tag {
			Ok(tag)
		} else {
			Err(error)
		}
	})?;
	Ok(tag.into())
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

fn read_ape(path: &Path) -> Result<SongTags> {
	let tag = ape::read(path)?;

	Ok(SongTags {
		artist: tag.item("Artist").and_then(read_ape_string),
		album_artist: tag.item("Album artist").and_then(read_ape_string),
		composer: tag.item("Composer").and_then(read_ape_string),
		lyricist: tag.item("Lyricist").and_then(read_ape_string),
		album: tag.item("Album").and_then(read_ape_string),
		title: tag.item("Title").and_then(read_ape_string),
		disc_number: tag.item("Disc").and_then(read_ape_x_of_y),
		track_number: tag.item("Track").and_then(read_ape_x_of_y),
		genre: tag.item("Genre").and_then(read_ape_string),
		year: tag.item("Year").and_then(read_ape_i32),
		duration: None,
		has_artwork: false,
	})
}

fn read_vorbis(path: &Path) -> Result<SongTags> {
	let file = fs::File::open(path)?;
	let source = OggStreamReader::new(file)?;

	let mut tags = SongTags {
		artist: None,
		album_artist: None,
		composer: None,
		lyricist: None,
		album: None,
		title: None,
		disc_number: None,
		track_number: None,
		genre: None,
		year: None,
		duration: None,
		has_artwork: false,
	};

	for (key, value) in source.comment_hdr.comment_list {
		utils::match_ignore_case! {
			match key {
				"ARTIST" => tags.artist = Some(value),
				"ALBUMARTIST" => tags.album_artist = Some(value),
				"COMPOSER" => tags.composer = Some(value),
				"LYRICIST" => tags.lyricist = Some(value),
				"ALBUM" => tags.album = Some(value),
				"TITLE" => tags.title = Some(value),
				"DISCNUMBER" => tags.disc_number = value.parse::<u32>().ok(),
				"TRACKNUMBER" => tags.track_number = value.parse::<u32>().ok(),
				"GENRE" => tags.genre = Some(value),
				"DATE" => tags.year = value.parse::<i32>().ok(),
				_ => (),
			}
		}
	}

	Ok(tags)
}

fn read_opus(path: &Path) -> Result<SongTags> {
	let headers = opus_headers::parse_from_path(path)?;

	let mut tags = SongTags {
		artist: None,
		album_artist: None,
		composer: None,
		lyricist: None,
		album: None,
		title: None,
		disc_number: None,
		track_number: None,
		genre: None,
		year: None,
		duration: None,
		has_artwork: false,
	};

	for (key, value) in headers.comments.user_comments {
		utils::match_ignore_case! {
			match key {
				"ARTIST" => tags.artist = Some(value),
				"ALBUMARTIST" => tags.album_artist = Some(value),
				"COMPOSER" => tags.composer = Some(value),
				"LYRICIST" => tags.lyricist = Some(value),
				"ALBUM" => tags.album = Some(value),
				"TITLE" => tags.title = Some(value),
				"DISCNUMBER" => tags.disc_number = value.parse::<u32>().ok(),
				"TRACKNUMBER" => tags.track_number = value.parse::<u32>().ok(),
				"GENRE" => tags.genre = Some(value),
				"DATE" => tags.year = value.parse::<i32>().ok(),
				_ => (),
			}
		}
	}

	Ok(tags)
}

fn read_flac(path: &Path) -> Result<SongTags> {
	let tag = metaflac::Tag::read_from_path(path)?;
	let vorbis = tag
		.vorbis_comments()
		.ok_or(anyhow!("Missing Vorbis comments"))?;
	let first_str = |val: Option<&Vec<_>>| val.and_then(|v| v.first().map(String::to_owned));

	let disc_number = vorbis
		.get("DISCNUMBER")
		.and_then(|v| v.first().and_then(|d| d.parse::<u32>().ok()));
	let year = vorbis
		.get("DATE")
		.and_then(|v| v.first().and_then(|d| d.parse::<i32>().ok()));

	let mut streaminfo = tag.get_blocks(metaflac::BlockType::StreamInfo);
	let duration = match streaminfo.next() {
		Some(&metaflac::Block::StreamInfo(ref s)) => {
			Some((s.total_samples as u32 / s.sample_rate) as u32)
		}
		_ => None,
	};

	Ok(SongTags {
		artist: first_str(vorbis.artist()),
		album_artist: first_str(vorbis.album_artist()),
		composer: first_str(vorbis.get("COMPOSER")),
		lyricist: first_str(vorbis.get("LYRICIST")),
		album: first_str(vorbis.album()),
		title: first_str(vorbis.title()),
		disc_number,
		track_number: vorbis.track(),
		genre: first_str(vorbis.genre()),
		year,
		duration,
		has_artwork: tag.pictures().count() > 0,
	})
}

fn read_mp4(path: &Path) -> Result<SongTags> {
	let mut tag = mp4ameta::Tag::read_from_path(path)?;

	Ok(SongTags {
		artist: tag.take_artist(),
		album_artist: tag.take_album_artist(),
		composer: tag.take_composer(),
		lyricist: tag.take_lyricist(),
		album: tag.take_album(),
		title: tag.take_title(),
		disc_number: tag.disc_number().map(|d| d as u32),
		track_number: tag.track_number().map(|d| d as u32),
		genre: tag.take_genre(),
		year: tag.year().and_then(|v| v.parse::<i32>().ok()),
		duration: tag.duration().map(|d| d.as_secs() as u32),
		has_artwork: tag.artwork().is_some(),
	})
}

#[test]
fn reads_file_metadata() {
	let sample_tags = SongTags {
		artist: Some("TEST ARTIST".into()),
		album_artist: Some("TEST ALBUM ARTIST".into()),
		composer: None,
		lyricist: None,
		album: Some("TEST ALBUM".into()),
		title: Some("TEST TITLE".into()),
		disc_number: Some(3),
		track_number: Some(1),
		duration: None,
		genre: None,
		year: Some(2016),
		has_artwork: false,
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
		read(Path::new("test-data/formats/sample.aif")).unwrap(),
		sample_tags
	);
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
	assert_eq!(
		read(Path::new("test-data/formats/sample.wav")).unwrap(),
		sample_tags
	);
}

#[test]
fn reads_embedded_artwork() {
	assert!(
		read(Path::new("test-data/artwork/sample.aif"))
			.unwrap()
			.has_artwork
	);
	assert!(
		read(Path::new("test-data/artwork/sample.mp3"))
			.unwrap()
			.has_artwork
	);
	assert!(
		read(Path::new("test-data/artwork/sample.flac"))
			.unwrap()
			.has_artwork
	);
	assert!(
		read(Path::new("test-data/artwork/sample.m4a"))
			.unwrap()
			.has_artwork
	);
	assert!(
		read(Path::new("test-data/artwork/sample.wav"))
			.unwrap()
			.has_artwork
	);
}
