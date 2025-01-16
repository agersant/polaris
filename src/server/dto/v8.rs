use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::app::{config, index, peaks, playlist, scanner, thumbnail};
use std::{collections::HashMap, convert::From, path::PathBuf, time::UNIX_EPOCH};

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize, ToSchema)]
pub struct Version {
	#[schema(examples(8))]
	pub major: i32,
	#[schema(examples(0))]
	pub minor: i32,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize, ToSchema)]
pub struct InitialSetup {
	#[schema(examples(true, false))]
	pub has_any_users: bool,
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct Credentials {
	#[schema(examples("alice"))]
	pub username: String,
	#[schema(examples("secret_password!!"))]
	pub password: String,
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct Authorization {
	#[schema(examples("alice"))]
	pub username: String,
	#[schema(
		examples("2U9OOdG2xAblxbhX1EhhjnjJJhw9SAeN1jIVdJ8UYGBBjgD73xeSFHECiYsB7ueBPwJ9ljR4WjlxU0jvcUw94LWbX2OHINKyvCneQgcf5YxjuXI8RTdqrxxTrpjR19p")
	)]
	pub token: String,
	#[schema(examples(true, false))]
	pub is_admin: bool,
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthQueryParameters {
	#[schema(
		examples("2U9OOdG2xAblxbhX1EhhjnjJJhw9SAeN1jIVdJ8UYGBBjgD73xeSFHECiYsB7ueBPwJ9ljR4WjlxU0jvcUw94LWbX2OHINKyvCneQgcf5YxjuXI8RTdqrxxTrpjR19p")
	)]
	pub auth_token: String,
}

#[derive(Serialize, Deserialize, IntoParams, ToSchema)]
pub struct ThumbnailOptions {
	pub size: Option<ThumbnailSize>,
	#[schema(examples(true, false))]
	pub pad: Option<bool>,
}

impl From<ThumbnailOptions> for thumbnail::Options {
	fn from(dto: ThumbnailOptions) -> Self {
		let mut options = thumbnail::Options::default();
		options.max_dimension = dto.size.map_or(options.max_dimension, Into::into);
		options.pad_to_square = dto.pad.unwrap_or(options.pad_to_square);
		options
	}
}

#[derive(Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(example = "small")]
pub enum ThumbnailSize {
	Tiny,
	Small,
	Large,
	Native,
}

#[allow(clippy::from_over_into)]
impl Into<Option<u32>> for ThumbnailSize {
	fn into(self) -> Option<u32> {
		match self {
			Self::Tiny => Some(40),
			Self::Small => Some(400),
			Self::Large => Some(1200),
			Self::Native => None,
		}
	}
}

pub type Peaks = Vec<u8>;

impl From<peaks::Peaks> for Peaks {
	fn from(p: peaks::Peaks) -> Self {
		p.interleaved
	}
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PlaylistHeader {
	#[schema(examples("Hotel Lounge Jazz", "Chill Beats 🏝️"))]
	pub name: String,
	#[schema(examples(json!({ "Jazz": 2, "Classical": 11 })))]
	pub num_songs_by_genre: HashMap<String, u32>,
	#[schema(examples(2309))]
	/// Playlist duration in seconds
	pub duration: u64,
}

impl From<playlist::PlaylistHeader> for PlaylistHeader {
	fn from(header: playlist::PlaylistHeader) -> Self {
		Self {
			name: header.name.to_string(),
			num_songs_by_genre: header.num_songs_by_genre,
			duration: header.duration.as_secs(),
		}
	}
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Playlist {
	#[serde(flatten)]
	pub header: PlaylistHeader,
	pub songs: SongList,
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct SavePlaylistInput {
	#[schema(value_type = Vec<String>, examples(json!(["my_music/destiny.mp3", "my_music/dancing_all_night.mp3"])))]
	pub tracks: Vec<PathBuf>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct User {
	#[schema(examples("alice"))]
	pub name: String,
	#[schema(examples(true, false))]
	pub is_admin: bool,
}

impl From<config::User> for User {
	fn from(u: config::User) -> Self {
		Self {
			name: u.name,
			is_admin: u.admin == Some(true),
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct NewUser {
	#[schema(examples("alice"))]
	pub name: String,
	#[schema(examples("secret-password!!"))]
	pub password: String,
	#[schema(examples(true, false))]
	pub admin: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct UserUpdate {
	#[schema(examples("secret-password!!"))]
	pub new_password: Option<String>,
	#[schema(examples(true, false))]
	pub new_is_admin: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, ToSchema)]
pub struct MountDir {
	#[schema(value_type = String, examples("/home/alice/music", "C:\\Users\\alice\\Documents\\Music"))]
	pub source: PathBuf,
	#[schema(examples("my_music", "root"))]
	pub name: String,
}

impl From<MountDir> for config::storage::MountDir {
	fn from(m: MountDir) -> Self {
		Self {
			name: m.name,
			source: m.source,
		}
	}
}

impl From<config::MountDir> for MountDir {
	fn from(m: config::MountDir) -> Self {
		Self {
			name: m.name,
			source: m.source,
		}
	}
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct NewSettings {
	#[schema(examples("Folder.(jpeg|jpg|png)"))]
	pub album_art_pattern: Option<String>,
	#[schema(examples("https://myddnsprovider.com?token=abcdef"))]
	pub ddns_update_url: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Settings {
	#[schema(examples("Folder.(jpeg|jpg|png)"))]
	pub album_art_pattern: String,
	#[schema(examples("https://myddnsprovider.com?token=abcdef"))]
	pub ddns_update_url: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum IndexState {
	OutOfDate,
	InProgress,
	UpToDate,
}

impl From<scanner::State> for IndexState {
	fn from(state: scanner::State) -> Self {
		match state {
			scanner::State::Initial => Self::OutOfDate,
			scanner::State::Pending => Self::OutOfDate,
			scanner::State::InProgress => Self::InProgress,
			scanner::State::UpToDate => Self::UpToDate,
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct IndexStatus {
	state: IndexState,
	#[schema(examples(1736929092))]
	last_start_time: Option<u64>,
	#[schema(examples(1736929992))]
	last_end_time: Option<u64>,
	#[schema(examples(289))]
	num_songs_indexed: u32,
}

impl From<scanner::Status> for IndexStatus {
	fn from(s: scanner::Status) -> Self {
		Self {
			state: s.state.into(),
			last_start_time: s
				.last_start_time
				.and_then(|t| t.duration_since(UNIX_EPOCH).ok())
				.map(|d| d.as_millis() as u64),
			last_end_time: s
				.last_end_time
				.and_then(|t| t.duration_since(UNIX_EPOCH).ok())
				.map(|d| d.as_millis() as u64),
			num_songs_indexed: s.num_songs_indexed,
		}
	}
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Song {
	#[schema(value_type = String, examples("my_music/destiny.mp3"))]
	pub path: PathBuf,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	#[schema(examples(1))]
	pub track_number: Option<i64>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	#[schema(examples(1))]
	pub disc_number: Option<i64>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	#[schema(examples("Destiny", "Dancing All Night"))]
	pub title: Option<String>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	#[schema(examples(json!(["Stratovarius"]), json!(["Cool Groove", "Smooth Doot"])))]
	pub artists: Vec<String>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	#[schema(examples(json!(["Stratovarius"]), json!(["Various Artists"])))]
	pub album_artists: Vec<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	#[schema(examples(2018))]
	pub year: Option<i64>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	#[schema(examples("Swing Tunes"))]
	pub album: Option<String>,
	#[schema(value_type = Option<String>)]
	#[serde(default, skip_serializing_if = "Option::is_none")]
	#[schema(examples("/my_music/destiny.jpg"))]
	pub artwork: Option<PathBuf>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	/// Duration in seconds
	#[schema(examples(192))]
	pub duration: Option<i64>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	#[schema(examples(json!(["John Writer", "Isabel Editor"])))]
	pub lyricists: Vec<String>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	#[schema(examples(json!(["Jane Composer"])))]
	pub composers: Vec<String>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	#[schema(examples(json!(["Jazz", "Classical"])))]
	pub genres: Vec<String>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	#[schema(examples(json!(["Ninja Tuna"])))]
	pub labels: Vec<String>,
}

impl From<index::Song> for Song {
	fn from(s: index::Song) -> Self {
		Self {
			path: s.virtual_path,
			track_number: s.track_number,
			disc_number: s.disc_number,
			title: s.title,
			artists: s.artists,
			album_artists: s.album_artists,
			year: s.year,
			album: s.album,
			artwork: s.artwork,
			duration: s.duration,
			lyricists: s.lyricists,
			composers: s.composers,
			genres: s.genres,
			labels: s.labels,
		}
	}
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SongList {
	#[schema(value_type = Vec<String>, examples(json!(["my_music/destiny.mp3", "my_music/sos.mp3"])))]
	pub paths: Vec<PathBuf>,
	/// Detailed metadata about the first few hundred songs listed in `.paths`
	pub first_songs: Vec<Song>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct BrowserEntry {
	#[schema(value_type = String, examples("my_music/stratovarius/destiny"))]
	pub path: PathBuf,
	#[schema(examples(true, false))]
	pub is_directory: bool,
}

impl From<index::File> for BrowserEntry {
	fn from(file: index::File) -> Self {
		match file {
			index::File::Directory(d) => Self {
				is_directory: true,
				path: d,
			},
			index::File::Song(s) => Self {
				is_directory: false,
				path: s,
			},
		}
	}
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct GenreHeader {
	#[schema(examples("Jazz", "Classical"))]
	pub name: String,
}

impl From<index::GenreHeader> for GenreHeader {
	fn from(g: index::GenreHeader) -> Self {
		Self {
			name: g.name.to_string(),
		}
	}
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Genre {
	#[serde(flatten)]
	pub header: GenreHeader,
	#[schema(examples(json!({ "Jazz": 20, "Classical": 90 })))]
	pub related_genres: HashMap<String, u32>,
	pub main_artists: Vec<ArtistHeader>,
	pub recently_added: Vec<AlbumHeader>,
}

impl From<index::Genre> for Genre {
	fn from(mut genre: index::Genre) -> Self {
		let main_artists = {
			genre.artists.sort_by_key(|a| {
				-(a.num_songs_by_genre
					.get(&genre.header.name)
					.copied()
					.unwrap_or_default() as i32)
			});
			genre
				.artists
				.into_iter()
				.take(20)
				.map(|a| a.into())
				.collect()
		};

		let recently_added = {
			genre.albums.sort_by_key(|a| -a.date_added);
			genre
				.albums
				.into_iter()
				.take(20)
				.map(|a| a.into())
				.collect()
		};

		Self {
			header: GenreHeader::from(genre.header),
			related_genres: genre.related_genres,
			main_artists,
			recently_added,
		}
	}
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ArtistHeader {
	#[schema(examples("Stratovarius", "Parov Stelar"))]
	pub name: String,
	#[schema(examples(0, 5))]
	pub num_albums_as_performer: u32,
	#[schema(examples(0, 5))]
	pub num_albums_as_additional_performer: u32,
	#[schema(examples(0, 5))]
	pub num_albums_as_composer: u32,
	#[schema(examples(0, 5))]
	pub num_albums_as_lyricist: u32,
	#[schema(examples(json!({ "Jazz": 2, "Classical": 11 })))]
	pub num_songs_by_genre: HashMap<String, u32>,
	#[schema(examples(12))]
	pub num_songs: u32,
}

impl From<index::ArtistHeader> for ArtistHeader {
	fn from(a: index::ArtistHeader) -> Self {
		Self {
			name: a.name.to_string(),
			num_albums_as_performer: a.num_albums_as_performer,
			num_albums_as_additional_performer: a.num_albums_as_additional_performer,
			num_albums_as_composer: a.num_albums_as_composer,
			num_albums_as_lyricist: a.num_albums_as_lyricist,
			num_songs_by_genre: a.num_songs_by_genre,
			num_songs: a.num_songs,
		}
	}
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Artist {
	#[serde(flatten)]
	pub header: ArtistHeader,
	pub albums: Vec<ArtistAlbum>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ArtistAlbum {
	#[serde(flatten)]
	pub album: AlbumHeader,
	pub contributions: Vec<Contribution>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Contribution {
	#[schema(examples(true, false))]
	pub performer: bool,
	#[schema(examples(true, false))]
	pub composer: bool,
	#[schema(examples(true, false))]
	pub lyricist: bool,
}

impl From<index::Artist> for Artist {
	fn from(artist: index::Artist) -> Self {
		let artist_name = artist.header.name.clone();
		let convert_album = |album: index::Album| ArtistAlbum {
			contributions: album
				.songs
				.iter()
				.map(|song| Contribution {
					performer: song.artists.contains(&artist_name)
						|| song.album_artists.contains(&artist_name),
					composer: song.composers.contains(&artist_name),
					lyricist: song.lyricists.contains(&artist_name),
				})
				.collect(),
			album: AlbumHeader::from(album.header),
		};
		Self {
			header: ArtistHeader::from(artist.header),
			albums: artist.albums.into_iter().map(convert_album).collect(),
		}
	}
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AlbumHeader {
	#[schema(examples("Destiny", "Swing Tunes"))]
	pub name: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	#[schema(value_type = String, examples("my_music/destiny.jpg"))]
	pub artwork: Option<PathBuf>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	#[schema(examples(json!(["Stratovarius"]), json!(["Various Artists"]), json!(["Shirley Music", "Chris Solo"])))]
	pub main_artists: Vec<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	#[schema(examples(2010, 2024))]
	pub year: Option<i64>,
}

impl From<index::AlbumHeader> for AlbumHeader {
	fn from(a: index::AlbumHeader) -> Self {
		Self {
			name: a.name,
			artwork: a.artwork,
			main_artists: a.artists,
			year: a.year,
		}
	}
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Album {
	#[serde(flatten)]
	pub header: AlbumHeader,
	pub songs: Vec<Song>,
}

impl From<index::Album> for Album {
	fn from(mut a: index::Album) -> Self {
		let songs = a.songs.drain(..).map(|s| s.into()).collect();
		Self {
			header: a.header.into(),
			songs: songs,
		}
	}
}

#[derive(Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct GetSongsBulkInput {
	#[schema(value_type = Vec<String>, examples(json!(["my_music/destiny.mp3", "my_music/sos.mp3"])))]
	pub paths: Vec<PathBuf>,
}

#[derive(Default, Serialize, Deserialize, ToSchema)]
pub struct GetSongsBulkOutput {
	pub songs: Vec<Song>,
	/// Path to requested songs that could not be found in the collection
	#[schema(value_type = Vec<String>, examples(json!(["my_music/destiny.mp3", "my_music/sos.mp3"])))]
	pub not_found: Vec<PathBuf>,
}

#[derive(Clone, Serialize, Deserialize, IntoParams, ToSchema)]
pub struct GetRandomAlbumsParameters {
	#[schema(examples(976878))]
	pub seed: Option<u64>,
	#[schema(examples(0, 100))]
	pub offset: Option<usize>,
	#[schema(examples(100, 1000))]
	pub count: Option<usize>,
}

#[derive(Clone, Serialize, Deserialize, IntoParams, ToSchema)]
pub struct GetRecentAlbumsParameters {
	#[schema(examples(0, 100))]
	pub offset: Option<usize>,
	#[schema(examples(100, 1000))]
	pub count: Option<usize>,
}
