use ape;
use id3::Tag;
use lewton::inside_ogg::OggStreamReader;
use ogg::PacketReader;
use regex::Regex;
use std::fs;
use std::path::Path;

use error::PError;
use utils;
use utils::AudioFormat;

pub struct SongTags {
    pub disc_number: Option<u32>,
    pub track_number: Option<u32>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album_artist: Option<String>,
    pub album: Option<String>,
    pub year: Option<i32>,
}

impl SongTags {
    pub fn read(path: &Path) -> Result<SongTags, PError> {
        match utils::get_audio_format(path) {
            Some(AudioFormat::MP3) => SongTags::read_id3(path),
            Some(AudioFormat::MPC) => SongTags::read_ape(path),
            Some(AudioFormat::OGG) => SongTags::read_vorbis(path),
            _ => Err(PError::UnsupportedMetadataFormat),
        }
    }

    fn read_id3(path: &Path) -> Result<SongTags, PError> {
        let tag = try!(Tag::read_from_path(path));

        let artist = tag.artist().map(|s| s.to_string());
        let album_artist = tag.album_artist().map(|s| s.to_string());
        let album = tag.album().map(|s| s.to_string());
        let title = tag.title().map(|s| s.to_string());
        let disc_number = tag.disc();
        let track_number = tag.track();
        let year = tag.year()
            .map(|y| y as i32)
            .or(tag.date_released().and_then(|d| d.year))
            .or(tag.date_recorded().and_then(|d| d.year));

        Ok(SongTags {
            artist: artist,
            album_artist: album_artist,
            album: album,
            title: title,
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
                if let Some((start, end)) = format.find(s) {
                    s[start..end].parse().ok()
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn read_ape(path: &Path) -> Result<SongTags, PError> {
        let tag = try!(ape::read(path));
        let artist = tag.item("Artist").and_then(SongTags::read_ape_string);
        let album = tag.item("Album").and_then(SongTags::read_ape_string);
        let album_artist = tag.item("Album artist").and_then(SongTags::read_ape_string);
        let title = tag.item("Title").and_then(SongTags::read_ape_string);
        let year = tag.item("Year").and_then(SongTags::read_ape_i32);
        let disc_number = tag.item("Disc").and_then(SongTags::read_ape_x_of_y);
        let track_number = tag.item("Track").and_then(SongTags::read_ape_x_of_y);
        Ok(SongTags {
            artist: artist,
            album_artist: album_artist,
            album: album,
            title: title,
            disc_number: disc_number,
            track_number: track_number,
            year: year,
        })
    }

    fn read_vorbis(path: &Path) -> Result<SongTags, PError> {

        let file = try!(fs::File::open(path));
        let source = try!(OggStreamReader::new(PacketReader::new(file)));

        let mut tags = SongTags {
            artist: None,
            album_artist: None,
            album: None,
            title: None,
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
}