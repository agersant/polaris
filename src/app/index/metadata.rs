use id3::TagLike;
use lewton::inside_ogg::OggStreamReader;
use log::error;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

use crate::utils;
use crate::utils::AudioFormat;

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error(transparent)]
	Ape(#[from] ape::Error),
	#[error(transparent)]
	Id3(#[from] id3::Error),
	#[error("Filesystem error for `{0}`: `{1}`")]
	Io(PathBuf, std::io::Error),
	#[error(transparent)]
	Metaflac(#[from] metaflac::Error),
	#[error(transparent)]
	Mp4aMeta(#[from] mp4ameta::Error),
	#[error(transparent)]
	Opus(#[from] opus_headers::ParseError),
	#[error(transparent)]
	Vorbis(#[from] lewton::VorbisError),
	#[error("Could not find a Vorbis comment within flac file")]
	VorbisCommentNotFoundInFlacFile,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SongTags {
	pub disc_number: Option<u32>,
	pub track_number: Option<u32>,
	pub title: Option<String>,
	pub duration: Option<u32>,
	pub artists: Vec<String>,
	pub album_artists: Vec<String>,
	pub album: Option<String>,
	pub year: Option<i32>,
	pub has_artwork: bool,
	pub lyricist: Option<String>,
	pub composer: Option<String>,
	pub genre: Option<String>,
	pub label: Option<String>,
}

impl From<id3::Tag> for SongTags {
	fn from(tag: id3::Tag) -> Self {
		SongTags {
			disc_number: tag.disc(),
			track_number: tag.track(),
			title: tag.title().map(|s| s.to_string()),
			duration: tag.duration(),
			artists: tag
				.artists()
				.map_or(Vec::new(), |v| v.iter().map(|s| s.to_string()).collect()),
			album_artists: tag
				.text_values_for_frame_id("TPE2")
				.map_or(Vec::new(), |v| v.iter().map(|s| s.to_string()).collect()),
			album: tag.album().map(|s| s.to_string()),
			year: tag
				.year()
				.map(|y| y as i32)
				.or_else(|| tag.date_released().map(|d| d.year))
				.or_else(|| tag.original_date_released().map(|d| d.year))
				.or_else(|| tag.date_recorded().map(|d| d.year)),
			has_artwork: tag.pictures().count() > 0,
			lyricist: tag.get_text("TEXT"),
			composer: tag.get_text("TCOM"),
			genre: tag.genre().map(|s| s.to_string()),
			label: tag.get_text("TPUB"),
		}
	}
}

pub fn read(path: &Path) -> Option<SongTags> {
	let data = match utils::get_audio_format(path) {
		Some(AudioFormat::AIFF) => read_aiff(path),
		Some(AudioFormat::APE) => read_ape(path),
		Some(AudioFormat::FLAC) => read_flac(path),
		Some(AudioFormat::MP3) => read_mp3(path),
		Some(AudioFormat::MP4) => read_mp4(path),
		Some(AudioFormat::MPC) => read_ape(path),
		Some(AudioFormat::OGG) => read_vorbis(path),
		Some(AudioFormat::OPUS) => read_opus(path),
		Some(AudioFormat::WAVE) => read_wave(path),
		None => return None,
	};
	match data {
		Ok(d) => Some(d),
		Err(e) => {
			error!("Error while reading file metadata for '{:?}': {}", path, e);
			None
		}
	}
}

trait FrameContent {
	/// Returns the value stored, if any, in the Frame.
	/// Say "TCOM" returns composer field.
	fn get_text(&self, key: &str) -> Option<String>;
}

impl FrameContent for id3::Tag {
	fn get_text(&self, key: &str) -> Option<String> {
		let frame = self.get(key)?;
		match frame.content() {
			id3::Content::Text(value) => Some(value.to_string()),
			_ => None,
		}
	}
}

fn read_mp3(path: &Path) -> Result<SongTags, Error> {
	let tag = id3::Tag::read_from_path(path).or_else(|error| {
		if let Some(tag) = error.partial_tag {
			Ok(tag)
		} else {
			Err(error)
		}
	})?;

	let duration = {
		mp3_duration::from_path(path)
			.map(|d| d.as_secs() as u32)
			.ok()
	};

	let mut song_tags: SongTags = tag.into();
	song_tags.duration = duration; // Use duration from mp3_duration instead of from tags.
	Ok(song_tags)
}

fn read_aiff(path: &Path) -> Result<SongTags, Error> {
	let tag = id3::Tag::read_from_aiff_path(path).or_else(|error| {
		if let Some(tag) = error.partial_tag {
			Ok(tag)
		} else {
			Err(error)
		}
	})?;
	Ok(tag.into())
}

fn read_wave(path: &Path) -> Result<SongTags, Error> {
	let tag = id3::Tag::read_from_wav_path(path).or_else(|error| {
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

fn read_ape(path: &Path) -> Result<SongTags, Error> {
	let tag = ape::read_from_path(path)?;

	let mut tags = SongTags::default();
	for item in tag.iter() {
		let key = item.key.as_str();
		utils::match_ignore_case! {
			match key {
				"TITLE" => tags.title = read_ape_string(item),
				"ALBUM" => tags.album = read_ape_string(item),
				"ARTIST" => tags.artists.extend(read_ape_string(item)),
				"ALBUM ARTIST" => tags.album_artists.extend(read_ape_string(item)),
				"TRACK" => tags.track_number = read_ape_x_of_y(item),
				"DISC" => tags.disc_number = read_ape_x_of_y(item),
				"YEAR" => tags.year = read_ape_i32(item),
				"LYRICIST" => tags.lyricist = read_ape_string(item),
				"COMPOSER" => tags.composer = read_ape_string(item),
				"GENRE" => tags.genre = read_ape_string(item),
				"PUBLISHER" => tags.label = read_ape_string(item),
				_ => (),
			}
		}
	}
	Ok(tags)
}

fn read_vorbis(path: &Path) -> Result<SongTags, Error> {
	let file = fs::File::open(path).map_err(|e| Error::Io(path.to_owned(), e))?;
	let source = OggStreamReader::new(file)?;

	let mut tags = SongTags::default();
	for (key, value) in source.comment_hdr.comment_list {
		utils::match_ignore_case! {
			match key {
				"TITLE" => tags.title = Some(value),
				"ALBUM" => tags.album = Some(value),
				"ARTIST" => tags.artists.push(value),
				"ALBUMARTIST" => tags.album_artists.push(value),
				"TRACKNUMBER" => tags.track_number = value.parse::<u32>().ok(),
				"DISCNUMBER" => tags.disc_number = value.parse::<u32>().ok(),
				"DATE" => tags.year = value.parse::<i32>().ok(),
				"LYRICIST" => tags.lyricist = Some(value),
				"COMPOSER" => tags.composer = Some(value),
				"GENRE" => tags.genre = Some(value),
				"PUBLISHER" => tags.label = Some(value),
				_ => (),
			}
		}
	}

	Ok(tags)
}

fn read_opus(path: &Path) -> Result<SongTags, Error> {
	let headers = opus_headers::parse_from_path(path)?;

	let mut tags = SongTags::default();
	for (key, value) in headers.comments.user_comments {
		utils::match_ignore_case! {
			match key {
				"TITLE" => tags.title = Some(value),
				"ALBUM" => tags.album = Some(value),
				"ARTIST" => tags.artists.push(value),
				"ALBUMARTIST" => tags.album_artists.push(value),
				"TRACKNUMBER" => tags.track_number = value.parse::<u32>().ok(),
				"DISCNUMBER" => tags.disc_number = value.parse::<u32>().ok(),
				"DATE" => tags.year = value.parse::<i32>().ok(),
				"LYRICIST" => tags.lyricist = Some(value),
				"COMPOSER" => tags.composer = Some(value),
				"GENRE" => tags.genre = Some(value),
				"PUBLISHER" => tags.label = Some(value),
				_ => (),
			}
		}
	}

	Ok(tags)
}

fn read_flac(path: &Path) -> Result<SongTags, Error> {
	let tag = metaflac::Tag::read_from_path(path)?;
	let vorbis = tag
		.vorbis_comments()
		.ok_or(Error::VorbisCommentNotFoundInFlacFile)?;
	let disc_number = vorbis
		.get("DISCNUMBER")
		.and_then(|d| d[0].parse::<u32>().ok());
	let mut streaminfo = tag.get_blocks(metaflac::BlockType::StreamInfo);
	let duration = match streaminfo.next() {
		Some(metaflac::Block::StreamInfo(s)) => Some(s.total_samples as u32 / s.sample_rate),
		_ => None,
	};
	Ok(SongTags {
		artists: vorbis.artist().map_or(Vec::new(), Vec::clone),
		album_artists: vorbis.album_artist().map_or(Vec::new(), Vec::clone),
		album: vorbis.album().map(|v| v[0].clone()),
		title: vorbis.title().map(|v| v[0].clone()),
		duration,
		disc_number,
		track_number: vorbis.track(),
		year: vorbis.get("DATE").and_then(|d| d[0].parse::<i32>().ok()),
		has_artwork: tag.pictures().count() > 0,
		lyricist: vorbis.get("LYRICIST").map(|v| v[0].clone()),
		composer: vorbis.get("COMPOSER").map(|v| v[0].clone()),
		genre: vorbis.get("GENRE").map(|v| v[0].clone()),
		label: vorbis.get("PUBLISHER").map(|v| v[0].clone()),
	})
}

fn read_mp4(path: &Path) -> Result<SongTags, Error> {
	let mut tag = mp4ameta::Tag::read_from_path(path)?;
	let label_ident = mp4ameta::FreeformIdent::new("com.apple.iTunes", "Label");

	Ok(SongTags {
		artists: tag.take_artists().collect(),
		album_artists: tag.take_album_artists().collect(),
		album: tag.take_album(),
		title: tag.take_title(),
		duration: tag.duration().map(|v| v.as_secs() as u32),
		disc_number: tag.disc_number().map(|d| d as u32),
		track_number: tag.track_number().map(|d| d as u32),
		year: tag.year().and_then(|v| v.parse::<i32>().ok()),
		has_artwork: tag.artwork().is_some(),
		lyricist: tag.take_lyricist(),
		composer: tag.take_composer(),
		genre: tag.take_genre(),
		label: tag.take_strings_of(&label_ident).next(),
	})
}

#[test]
fn reads_file_metadata() {
	let sample_tags = SongTags {
		disc_number: Some(3),
		track_number: Some(1),
		title: Some("TEST TITLE".into()),
		artists: vec!["TEST ARTIST".into()],
		album_artists: vec!["TEST ALBUM ARTIST".into()],
		album: Some("TEST ALBUM".into()),
		duration: None,
		year: Some(2016),
		has_artwork: false,
		lyricist: Some("TEST LYRICIST".into()),
		composer: Some("TEST COMPOSER".into()),
		genre: Some("TEST GENRE".into()),
		label: Some("TEST LABEL".into()),
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
