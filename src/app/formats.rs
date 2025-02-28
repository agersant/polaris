use id3::TagLike;
use lewton::inside_ogg::OggStreamReader;
use log::error;
use std::fs;
use std::io::{Seek, SeekFrom};
use std::path::Path;

use crate::app::Error;
use crate::utils;
use crate::utils::AudioFormat;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SongMetadata {
	pub disc_number: Option<u32>,
	pub track_number: Option<u32>,
	pub title: Option<String>,
	pub duration: Option<u32>,
	pub artists: Vec<String>,
	pub album_artists: Vec<String>,
	pub album: Option<String>,
	pub year: Option<i32>,
	pub has_artwork: bool,
	pub lyricists: Vec<String>,
	pub composers: Vec<String>,
	pub genres: Vec<String>,
	pub labels: Vec<String>,
}

pub fn read_metadata<P: AsRef<Path>>(path: P) -> Option<SongMetadata> {
	let data = match utils::get_audio_format(&path) {
		Some(AudioFormat::AIFF) => read_id3(&path),
		Some(AudioFormat::FLAC) => read_flac(&path),
		Some(AudioFormat::MP3) => read_mp3(&path),
		Some(AudioFormat::OGG) => read_vorbis(&path),
		Some(AudioFormat::OPUS) => read_opus(&path),
		Some(AudioFormat::WAVE) => read_id3(&path),
		Some(AudioFormat::APE) | Some(AudioFormat::MPC) => read_ape(&path),
		Some(AudioFormat::MP4) | Some(AudioFormat::M4B) => read_mp4(&path),
		None => return None,
	};
	match data {
		Ok(d) => Some(d),
		Err(e) => {
			error!(
				"Error while reading file metadata for '{:?}': {}",
				path.as_ref(),
				e
			);
			None
		}
	}
}

trait ID3Ext {
	fn get_text_values(&self, frame_name: &str) -> Vec<String>;
}

impl ID3Ext for id3::Tag {
	fn get_text_values(&self, frame_name: &str) -> Vec<String> {
		self.get(frame_name)
			.and_then(|f| f.content().text_values())
			.map(|i| i.map(str::to_string).collect())
			.unwrap_or_default()
	}
}

fn read_id3<P: AsRef<Path>>(path: P) -> Result<SongMetadata, Error> {
	let file = fs::File::open(path.as_ref()).map_err(|e| Error::Io(path.as_ref().to_owned(), e))?;
	read_id3_from_file(&file, path)
}

fn read_id3_from_file<P: AsRef<Path>>(file: &fs::File, path: P) -> Result<SongMetadata, Error> {
	let tag = id3::Tag::read_from2(file)
		.or_else(|error| {
			if let Some(tag) = error.partial_tag {
				Ok(tag)
			} else {
				Err(error)
			}
		})
		.map_err(|e| Error::Id3(path.as_ref().to_owned(), e))?;

	let artists = tag.get_text_values("TPE1");
	let album_artists = tag.get_text_values("TPE2");
	let album = tag.album().map(|s| s.to_string());
	let title = tag.title().map(|s| s.to_string());
	let duration = tag.duration();
	let disc_number = tag.disc();
	let track_number = tag.track();
	let year = tag
		.year()
		.or_else(|| tag.date_released().map(|d| d.year))
		.or_else(|| tag.original_date_released().map(|d| d.year))
		.or_else(|| tag.date_recorded().map(|d| d.year));
	let has_artwork = tag.pictures().count() > 0;
	let lyricists = tag.get_text_values("TEXT");
	let composers = tag.get_text_values("TCOM");
	let genres = tag.get_text_values("TCON");
	let labels = tag.get_text_values("TPUB");

	Ok(SongMetadata {
		disc_number,
		track_number,
		title,
		duration,
		artists,
		album_artists,
		album,
		year,
		has_artwork,
		lyricists,
		composers,
		genres,
		labels,
	})
}

fn read_mp3<P: AsRef<Path>>(path: P) -> Result<SongMetadata, Error> {
	let mut file = fs::File::open(&path).unwrap();
	let mut metadata = read_id3_from_file(&file, &path)?;
	metadata.duration = metadata.duration.or_else(|| {
		file.seek(SeekFrom::Start(0)).unwrap();
		mp3_duration::from_file(&file)
			.map(|d| d.as_secs() as u32)
			.ok()
	});
	Ok(metadata)
}

mod ape_ext {
	use regex::Regex;
	use std::sync::LazyLock;

	pub fn read_string(item: &ape::Item) -> Option<String> {
		item.try_into().ok().map(str::to_string)
	}

	pub fn read_strings(item: Option<&ape::Item>) -> Vec<String> {
		let Some(item) = item else {
			return vec![];
		};
		let strings: Vec<&str> = item.try_into().unwrap_or_default();
		strings.into_iter().map(str::to_string).collect()
	}

	pub fn read_i32(item: &ape::Item) -> Option<i32> {
		item.try_into()
			.ok()
			.and_then(|s: &str| s.parse::<i32>().ok())
	}

	static X_OF_Y_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^\d+"#).unwrap());

	pub fn read_x_of_y(item: &ape::Item) -> Option<u32> {
		item.try_into().ok().and_then(|s: &str| {
			if let Some(m) = X_OF_Y_REGEX.find(s) {
				s[m.start()..m.end()].parse().ok()
			} else {
				None
			}
		})
	}
}

fn read_ape<P: AsRef<Path>>(path: P) -> Result<SongMetadata, Error> {
	let tag = ape::read_from_path(path)?;
	let artists = ape_ext::read_strings(tag.item("Artist"));
	let album = tag.item("Album").and_then(ape_ext::read_string);
	let album_artists = ape_ext::read_strings(tag.item("Album artist"));
	let title = tag.item("Title").and_then(ape_ext::read_string);
	let year = tag.item("Year").and_then(ape_ext::read_i32);
	let disc_number = tag.item("Disc").and_then(ape_ext::read_x_of_y);
	let track_number = tag.item("Track").and_then(ape_ext::read_x_of_y);
	let lyricists = ape_ext::read_strings(tag.item("LYRICIST"));
	let composers = ape_ext::read_strings(tag.item("COMPOSER"));
	let genres = ape_ext::read_strings(tag.item("GENRE"));
	let labels = ape_ext::read_strings(tag.item("PUBLISHER"));
	Ok(SongMetadata {
		artists,
		album_artists,
		album,
		title,
		duration: None,
		disc_number,
		track_number,
		year,
		has_artwork: false,
		lyricists,
		composers,
		genres,
		labels,
	})
}

fn read_vorbis<P: AsRef<Path>>(path: P) -> Result<SongMetadata, Error> {
	let file = fs::File::open(&path).map_err(|e| Error::Io(path.as_ref().to_owned(), e))?;
	let source = OggStreamReader::new(file)?;

	let mut metadata = SongMetadata::default();
	for (key, value) in source.comment_hdr.comment_list {
		utils::match_ignore_case! {
			match key {
				"TITLE" => metadata.title = Some(value),
				"ALBUM" => metadata.album = Some(value),
				"ARTIST" => metadata.artists.push(value),
				"ALBUMARTIST" => metadata.album_artists.push(value),
				"TRACKNUMBER" => metadata.track_number = value.parse::<u32>().ok(),
				"DISCNUMBER" => metadata.disc_number = value.parse::<u32>().ok(),
				"DATE" => metadata.year = value.parse::<i32>().ok(),
				"LYRICIST" => metadata.lyricists.push(value),
				"COMPOSER" => metadata.composers.push(value),
				"GENRE" => metadata.genres.push(value),
				"PUBLISHER" => metadata.labels.push(value),
				_ => (),
			}
		}
	}

	Ok(metadata)
}

fn read_opus<P: AsRef<Path>>(path: P) -> Result<SongMetadata, Error> {
	let headers = opus_headers::parse_from_path(path)?;

	let mut metadata = SongMetadata::default();
	for (key, value) in headers.comments.user_comments {
		utils::match_ignore_case! {
			match key {
				"TITLE" => metadata.title = Some(value),
				"ALBUM" => metadata.album = Some(value),
				"ARTIST" => metadata.artists.push(value),
				"ALBUMARTIST" => metadata.album_artists.push(value),
				"TRACKNUMBER" => metadata.track_number = value.parse::<u32>().ok(),
				"DISCNUMBER" => metadata.disc_number = value.parse::<u32>().ok(),
				"DATE" => metadata.year = value.parse::<i32>().ok(),
				"LYRICIST" => metadata.lyricists.push(value),
				"COMPOSER" => metadata.composers.push(value),
				"GENRE" => metadata.genres.push(value),
				"PUBLISHER" => metadata.labels.push(value),
				_ => (),
			}
		}
	}

	Ok(metadata)
}

fn read_flac<P: AsRef<Path>>(path: P) -> Result<SongMetadata, Error> {
	let tag = metaflac::Tag::read_from_path(&path)
		.map_err(|e| Error::Metaflac(path.as_ref().to_owned(), e))?;
	let vorbis = tag
		.vorbis_comments()
		.ok_or(Error::VorbisCommentNotFoundInFlacFile)?;
	let disc_number = vorbis
		.get("DISCNUMBER")
		.and_then(|d| d[0].parse::<u32>().ok());
	let year = vorbis.get("DATE").and_then(|d| d[0].parse::<i32>().ok());
	let mut streaminfo = tag.get_blocks(metaflac::BlockType::StreamInfo);
	let duration = match streaminfo.next() {
		Some(metaflac::Block::StreamInfo(s)) => Some(s.total_samples as u32 / s.sample_rate),
		_ => None,
	};
	let has_artwork = tag.pictures().count() > 0;

	let multivalue = |o: Option<&Vec<String>>| o.cloned().unwrap_or_default();

	Ok(SongMetadata {
		artists: multivalue(vorbis.artist()),
		album_artists: multivalue(vorbis.album_artist()),
		album: vorbis.album().map(|v| v[0].clone()),
		title: vorbis.title().map(|v| v[0].clone()),
		duration,
		disc_number,
		track_number: vorbis.track(),
		year,
		has_artwork,
		lyricists: multivalue(vorbis.get("LYRICIST")),
		composers: multivalue(vorbis.get("COMPOSER")),
		genres: multivalue(vorbis.get("GENRE")),
		labels: multivalue(vorbis.get("PUBLISHER")),
	})
}

fn read_mp4<P: AsRef<Path>>(path: P) -> Result<SongMetadata, Error> {
	let cfg = mp4ameta::ReadConfig {
		read_meta_items: true,
		read_image_data: false,
		..mp4ameta::ReadConfig::NONE
	};
	let mut tag = mp4ameta::Tag::read_with_path(&path, &cfg)
		.map_err(|e| Error::Mp4aMeta(path.as_ref().to_owned(), e))?;
	let label_ident = mp4ameta::FreeformIdent::new_static("com.apple.iTunes", "LABEL");

	Ok(SongMetadata {
		artists: tag.take_artists().collect(),
		album_artists: tag.take_album_artists().collect(),
		album: tag.take_album(),
		title: tag.take_title(),
		duration: Some(tag.duration().as_secs() as u32),
		disc_number: tag.disc_number().map(|d| d as u32),
		track_number: tag.track_number().map(|d| d as u32),
		year: tag.year().and_then(|v| v.parse::<i32>().ok()),
		has_artwork: tag.artwork().is_some(),
		lyricists: tag.take_lyricists().collect(),
		composers: tag.take_composers().collect(),
		genres: tag.take_genres().collect(),
		labels: tag.take_strings_of(&label_ident).collect(),
	})
}

#[test]
fn reads_file_metadata() {
	let expected_without_duration = SongMetadata {
		disc_number: Some(3),
		track_number: Some(1),
		title: Some("TEST TITLE".into()),
		artists: vec!["TEST ARTIST".into()],
		album_artists: vec!["TEST ALBUM ARTIST".into()],
		album: Some("TEST ALBUM".into()),
		duration: None,
		year: Some(2016),
		has_artwork: false,
		lyricists: vec!["TEST LYRICIST".into()],
		composers: vec!["TEST COMPOSER".into()],
		genres: vec!["TEST GENRE".into()],
		labels: vec!["TEST LABEL".into()],
	};
	let expected_with_duration = SongMetadata {
		duration: Some(0),
		..expected_without_duration.clone()
	};
	assert_eq!(
		read_metadata(Path::new("test-data/formats/sample.aif")).unwrap(),
		expected_without_duration
	);
	assert_eq!(
		read_metadata(Path::new("test-data/formats/sample.mp3")).unwrap(),
		expected_with_duration
	);
	assert_eq!(
		read_metadata(Path::new("test-data/formats/sample.ogg")).unwrap(),
		expected_without_duration
	);
	assert_eq!(
		read_metadata(Path::new("test-data/formats/sample.flac")).unwrap(),
		expected_with_duration
	);
	assert_eq!(
		read_metadata(Path::new("test-data/formats/sample.m4a")).unwrap(),
		expected_with_duration
	);
	assert_eq!(
		read_metadata(Path::new("test-data/formats/sample.opus")).unwrap(),
		expected_without_duration
	);
	assert_eq!(
		read_metadata(Path::new("test-data/formats/sample.ape")).unwrap(),
		expected_without_duration
	);
	assert_eq!(
		read_metadata(Path::new("test-data/formats/sample.wav")).unwrap(),
		expected_without_duration
	);
}

#[test]
fn reads_embedded_artwork() {
	assert!(
		read_metadata(Path::new("test-data/artwork/sample.aif"))
			.unwrap()
			.has_artwork
	);
	assert!(
		read_metadata(Path::new("test-data/artwork/sample.mp3"))
			.unwrap()
			.has_artwork
	);
	assert!(
		read_metadata(Path::new("test-data/artwork/sample.flac"))
			.unwrap()
			.has_artwork
	);
	assert!(
		read_metadata(Path::new("test-data/artwork/sample.m4a"))
			.unwrap()
			.has_artwork
	);
	assert!(
		read_metadata(Path::new("test-data/artwork/sample.wav"))
			.unwrap()
			.has_artwork
	);
}

#[test]
fn reads_multivalue_fields() {
	let expected_without_duration = SongMetadata {
		disc_number: Some(3),
		track_number: Some(1),
		title: Some("TEST TITLE".into()),
		artists: vec!["TEST ARTIST".into(), "OTHER ARTIST".into()],
		album_artists: vec!["TEST ALBUM ARTIST".into(), "OTHER ALBUM ARTIST".into()],
		album: Some("TEST ALBUM".into()),
		duration: None,
		year: Some(2016),
		has_artwork: false,
		lyricists: vec!["TEST LYRICIST".into(), "OTHER LYRICIST".into()],
		composers: vec!["TEST COMPOSER".into(), "OTHER COMPOSER".into()],
		genres: vec!["TEST GENRE".into(), "OTHER GENRE".into()],
		labels: vec!["TEST LABEL".into(), "OTHER LABEL".into()],
	};
	let expected_with_duration = SongMetadata {
		duration: Some(0),
		..expected_without_duration.clone()
	};
	assert_eq!(
		read_metadata(Path::new("test-data/multivalue/multivalue.aif")).unwrap(),
		expected_without_duration
	);
	assert_eq!(
		read_metadata(Path::new("test-data/multivalue/multivalue.mp3")).unwrap(),
		expected_with_duration
	);
	assert_eq!(
		read_metadata(Path::new("test-data/multivalue/multivalue.ogg")).unwrap(),
		expected_without_duration
	);
	assert_eq!(
		read_metadata(Path::new("test-data/multivalue/multivalue.flac")).unwrap(),
		expected_with_duration
	);
	// TODO Test m4a support (likely working). Pending https://tickets.metabrainz.org/browse/PICARD-3029
	assert_eq!(
		read_metadata(Path::new("test-data/multivalue/multivalue.opus")).unwrap(),
		expected_without_duration
	);
	assert_eq!(
		read_metadata(Path::new("test-data/multivalue/multivalue.ape")).unwrap(),
		expected_without_duration
	);
	assert_eq!(
		read_metadata(Path::new("test-data/multivalue/multivalue.wav")).unwrap(),
		expected_without_duration
	);
}
