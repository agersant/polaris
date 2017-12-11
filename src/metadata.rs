use ape;
use id3;
use lewton::inside_ogg::OggStreamReader;
use metaflac;
use mp3_duration;
use regex::Regex;
use std::fs;
use std::path::Path;

use errors::*;
use utils;
use utils::AudioFormat;

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
}

pub fn read(path: &Path) -> Result<SongTags> {
	match utils::get_audio_format(path) {
		Some(AudioFormat::FLAC) => read_flac(path),
		Some(AudioFormat::MP3) => read_id3(path),
		Some(AudioFormat::MPC) => read_ape(path),
		Some(AudioFormat::OGG) => read_vorbis(path),
		_ => bail!("Unsupported file format for reading metadata"),
	}
}

fn read_id3(path: &Path) -> Result<SongTags> {
	let tag = id3::Tag::read_from_path(&path)?;
	let duration = mp3_duration::from_path(&path).map(|d| d.as_secs() as u32).ok();

	let artist = tag.artist().map(|s| s.to_string());
	let album_artist = tag.album_artist().map(|s| s.to_string());
	let album = tag.album().map(|s| s.to_string());
	let title = tag.title().map(|s| s.to_string());
	let disc_number = tag.disc();
	let track_number = tag.track();
	let year = tag.year()
		.map(|y| y as i32)
		.or(tag.date_released().and_then(|d| Some(d.year)))
		.or(tag.date_recorded().and_then(|d| Some(d.year)));

	Ok(SongTags {
	       artist: artist,
	       album_artist: album_artist,
	       album: album,
	       title: title,
		   duration: duration,
	       disc_number: disc_number,
	       track_number: track_number,
	       year: year,
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
	       artist: artist,
	       album_artist: album_artist,
	       album: album,
	       title: title,
		   duration: None,
	       disc_number: disc_number,
	       track_number: track_number,
	       year: year,
	   })
}

fn read_vorbis(path: &Path) -> Result<SongTags> {

	let file = fs::File::open(path)?;
	let source = OggStreamReader::new(file)?;

	let mut tags = SongTags {
		artist: None,
		album_artist: None,
		album: None,
		title: None,
		duration:None,
		disc_number: None,
		track_number: None,
		year: None,
	};

	for (key, value) in source.comment_hdr.comment_list {
		match key.as_str() {
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

	Ok(tags)
}

fn read_flac(path: &Path) -> Result<SongTags> {
	let tag = metaflac::Tag::read_from_path(path)?;
	let vorbis = tag.vorbis_comments().ok_or("Missing Vorbis comments")?;
	let disc_number = vorbis
		.get("DISCNUMBER")
		.and_then(|d| d[0].parse::<u32>().ok());
	let year = vorbis.get("DATE").and_then(|d| d[0].parse::<i32>().ok());
	let streaminfo = tag.get_blocks(metaflac::BlockType::StreamInfo);
	let duration = match streaminfo.first() {
        Some(&&metaflac::Block::StreamInfo(ref s)) => Some((s.total_samples as u32 / s.sample_rate) as u32),
        _ => None
	};

	Ok(SongTags {
	       artist: vorbis.artist().map(|v| v[0].clone()),
	       album_artist: vorbis.album_artist().map(|v| v[0].clone()),
	       album: vorbis.album().map(|v| v[0].clone()),
	       title: vorbis.title().map(|v| v[0].clone()),
	       duration: duration,
	       disc_number: disc_number,
	       track_number: vorbis.track(),
	       year: year,
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
	};
	let flac_sample_tag = SongTags {duration: Some(0), ..sample_tags.clone()};
	let mp3_sample_tag = SongTags {duration: Some(0), ..sample_tags.clone()};
	assert_eq!(read(Path::new("test/sample.mp3")).unwrap(), mp3_sample_tag);
	assert_eq!(read(Path::new("test/sample.ogg")).unwrap(), sample_tags);
	assert_eq!(read(Path::new("test/sample.flac")).unwrap(), flac_sample_tag);
}
