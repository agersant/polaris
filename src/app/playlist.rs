use core::clone::Clone;
use std::collections::{BTreeMap, HashMap};
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use icu_collator::{Collator, CollatorOptions, Strength};
use native_db::*;
use native_model::{native_model, Model};
use serde::{Deserialize, Serialize};
use tokio::task::spawn_blocking;
use zip::{write::FileOptions, ZipArchive, ZipWriter};

use crate::app::{config, index, ndb, Error};

#[derive(Clone)]
pub struct Manager {
	config_manager: config::Manager,
	index_manager: index::Manager,
	db: ndb::Manager,
}

#[derive(Debug)]
pub struct PlaylistHeader {
	pub name: String,
	pub duration: Duration,
	pub num_songs_by_genre: HashMap<String, u32>,
}

#[derive(Debug)]
pub struct Playlist {
	pub header: PlaylistHeader,
	pub songs: Vec<PathBuf>,
}

#[derive(Debug, Default)]
struct M3UPlaylist {
	title: Option<String>,
	songs: Vec<PathBuf>,
}

pub type PlaylistModel = v1::PlaylistModel;
type PlaylistModelKey = v1::PlaylistModelKey;

pub mod v1 {

	use super::*;

	#[derive(Debug, Default, Serialize, Deserialize)]
	#[native_model(id = 1, version = 1)]
	#[native_db(primary_key(custom_id -> (&str, &str)))]
	pub struct PlaylistModel {
		#[secondary_key]
		pub owner: String,
		pub name: String,
		pub duration: Duration,
		pub num_songs_by_genre: BTreeMap<String, u32>,
		pub virtual_paths: Vec<PathBuf>,
	}

	impl PlaylistModel {
		fn custom_id(&self) -> (&str, &str) {
			(&self.owner, &self.name)
		}
	}
}

impl From<PlaylistModel> for PlaylistHeader {
	fn from(p: PlaylistModel) -> Self {
		Self {
			name: p.name,
			duration: p.duration,
			num_songs_by_genre: p.num_songs_by_genre.into_iter().collect(),
		}
	}
}

impl From<PlaylistModel> for Playlist {
	fn from(mut p: PlaylistModel) -> Self {
		let songs = p.virtual_paths.drain(0..).collect();
		Self {
			songs,
			header: p.into(),
		}
	}
}

impl Manager {
	pub fn new(
		config_manager: config::Manager,
		index_manager: index::Manager,
		db: ndb::Manager,
	) -> Self {
		Self {
			config_manager,
			index_manager,
			db,
		}
	}

	pub async fn list_playlists(&self, owner: &str) -> Result<Vec<PlaylistHeader>, Error> {
		spawn_blocking({
			let manager = self.clone();
			let owner = owner.to_owned();
			move || {
				let transaction = manager.db.r_transaction()?;
				let mut playlists = transaction
					.scan()
					.secondary::<PlaylistModel>(PlaylistModelKey::owner)?
					.range(owner.as_str()..=owner.as_str())?
					.filter_map(|p| p.ok())
					.map(PlaylistHeader::from)
					.collect::<Vec<_>>();

				let collator_options = {
					let mut o = CollatorOptions::new();
					o.strength = Some(Strength::Secondary);
					o
				};
				let collator = Collator::try_new(&Default::default(), collator_options).unwrap();

				playlists.sort_by(|a, b| collator.compare(&a.name, &b.name));
				Ok(playlists)
			}
		})
		.await?
		.map_err(|e| Error::NativeDatabase(Box::new(e)))
	}

	pub async fn save_playlist(
		&self,
		name: &str,
		owner: &str,
		virtual_paths: Vec<PathBuf>,
	) -> Result<(), Error> {
		let songs = self
			.index_manager
			.get_songs(virtual_paths)
			.await
			.into_iter()
			.filter_map(|s| s.ok())
			.collect::<Vec<_>>();

		spawn_blocking({
			let manager = self.clone();
			let owner = owner.to_owned();
			let name = name.to_owned();
			move || {
				let transaction = manager.db.rw_transaction()?;

				let duration = songs
					.iter()
					.filter_map(|s| s.duration.map(|d| d as u64))
					.sum();

				let mut num_songs_by_genre = BTreeMap::<String, u32>::new();
				for song in &songs {
					for genre in &song.genres {
						*num_songs_by_genre.entry(genre.clone()).or_default() += 1;
					}
				}

				let virtual_paths = songs.into_iter().map(|s| s.virtual_path).collect();

				transaction.upsert::<PlaylistModel>(PlaylistModel {
					owner: owner.to_owned(),
					name: name.to_owned(),
					duration: Duration::from_secs(duration),
					num_songs_by_genre,
					virtual_paths,
				})?;

				transaction.commit()?;

				Ok(())
			}
		})
		.await?
		.map_err(|e| Error::NativeDatabase(Box::new(e)))
	}

	pub async fn read_playlist(&self, name: &str, owner: &str) -> Result<Playlist, Error> {
		spawn_blocking({
			let manager = self.clone();
			let owner = owner.to_owned();
			let name = name.to_owned();
			move || {
				let transaction = manager
					.db
					.r_transaction()
					.map_err(|e| Error::NativeDatabase(Box::new(e)))?;
				match transaction.get().primary::<PlaylistModel>((owner, name)) {
					Ok(Some(p)) => Ok(Playlist::from(p)),
					Ok(None) => Err(Error::PlaylistNotFound),
					Err(e) => Err(Error::NativeDatabase(Box::new(e))),
				}
			}
		})
		.await?
	}

	pub async fn delete_playlist(&self, name: &str, owner: &str) -> Result<(), Error> {
		spawn_blocking({
			let manager = self.clone();
			let owner = owner.to_owned();
			let name = name.to_owned();
			move || {
				let transaction = manager
					.db
					.rw_transaction()
					.map_err(|e| Error::NativeDatabase(Box::new(e)))?;
				let playlist = match transaction
					.get()
					.primary::<PlaylistModel>((owner.as_str(), name.as_str()))
				{
					Ok(Some(p)) => Ok(p),
					Ok(None) => Err(Error::PlaylistNotFound),
					Err(e) => Err(Error::NativeDatabase(Box::new(e))),
				}?;
				transaction
					.remove::<PlaylistModel>(playlist)
					.map_err(|e| Error::NativeDatabase(Box::new(e)))?;
				transaction
					.commit()
					.map_err(|e| Error::NativeDatabase(Box::new(e)))?;
				Ok(())
			}
		})
		.await?
	}

	pub async fn export_playlist(&self, name: &str, owner: &str) -> Result<Vec<u8>, Error> {
		let playlist = self.read_playlist(name, owner).await?;

		let mut m3u = String::with_capacity(playlist.songs.len() * 128);
		m3u.push_str(&format!("#PLAYLIST:{name}\n"));

		let real_paths = self
			.config_manager
			.resolve_virtual_paths(&playlist.songs)
			.await
			.into_iter()
			.collect::<Result<Vec<_>, _>>()?;

		for song in real_paths {
			m3u.push_str(&song.to_string_lossy());
			m3u.push('\n');
		}

		Ok(m3u.into_bytes())
	}

	pub async fn export_playlists(&self, owner: &str) -> Result<Vec<u8>, Error> {
		let playlists = self.list_playlists(owner).await?;

		let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
		for header in playlists {
			let name = &header.name;
			let exported = self.export_playlist(name, owner).await?;
			zip.start_file(format!("{owner}-{name}.m3u8"), FileOptions::DEFAULT)
				.or(Err(Error::PlaylistExportZip))?;
			zip.write_all(&exported).or(Err(Error::PlaylistExportZip))?;
		}

		let zipped = zip
			.finish_into_readable()
			.or(Err(Error::PlaylistExportZip))?;
		Ok(zipped.into_inner().into_inner())
	}

	pub async fn import_playlists(
		&self,
		owner: &str,
		files: HashMap<String, Vec<u8>>,
	) -> Result<(), Error> {
		let m3us = files
			.into_iter()
			.flat_map(|(name, data)| {
				if data.starts_with(&[0x50, 0x4B, 0x03, 0x04]) {
					read_zipped_playlists(&data)
				} else {
					match String::from_utf8(data).map_err(|_| Error::InvalidPlaylistTextEncoding) {
						Ok(data) => vec![Ok((name.to_owned(), data))],
						Err(e) => vec![Err(e)],
					}
				}
			})
			.collect::<Result<HashMap<String, String>, Error>>()?;

		for (filename, data) in m3us {
			let playlist = parse_m3u(&data);
			let title = playlist.title.unwrap_or(filename.to_owned());
			let virtual_paths = self
				.config_manager
				.virtualize_paths(&playlist.songs)
				.await
				.into_iter()
				.collect::<Result<Vec<_>, _>>()?;
			self.save_playlist(&title, owner, virtual_paths).await?;
		}

		Ok(())
	}
}

fn parse_m3u(data: &str) -> M3UPlaylist {
	let mut playlist = M3UPlaylist::default();

	for line in data.split_terminator('\n') {
		if line.starts_with("#PLAYLIST:") {
			if let Some((_, title)) = line.split_once(":") {
				playlist.title = Some(title.to_owned());
			}
		} else if line.starts_with("#") {
			continue;
		} else {
			playlist.songs.push(PathBuf::from_str(line).unwrap());
		}
	}

	playlist
}

fn read_zipped_playlists(data: &Vec<u8>) -> Vec<Result<(String, String), Error>> {
	let Ok(mut zip) = ZipArchive::new(Cursor::new(data)) else {
		return vec![Err(Error::PlaylistImportZip)];
	};

	(0..zip.len())
		.map(|i| {
			zip.by_index(i)
				.map_err(|_| Error::PlaylistImportZip)
				.and_then(|mut file| {
					let filename = file.name().to_owned();
					let mut content = String::new();
					file.read_to_string(&mut content)
						.map_err(|_| Error::InvalidPlaylistTextEncoding)?;
					Ok((filename, content))
				})
		})
		.collect()
}

#[cfg(test)]
mod test {
	use std::collections::HashMap;
	use std::path::PathBuf;

	use crate::app::test::{self, Context};
	use crate::test_name;

	const TEST_USER: &str = "test_user";
	const TEST_PASSWORD: &str = "password";
	const TEST_PLAYLIST_NAME: &str = "Chill & Grill";
	const TEST_MOUNT_NAME: &str = "root";

	async fn list_all_songs(ctx: &Context) -> Vec<PathBuf> {
		let paths = ctx
			.index_manager
			.flatten(PathBuf::from(TEST_MOUNT_NAME))
			.await
			.unwrap()
			.into_iter()
			.collect::<Vec<_>>();

		assert_eq!(paths.len(), 13);
		paths
	}

	#[tokio::test]
	async fn save_playlist_golden_path() {
		let ctx = test::ContextBuilder::new(test_name!())
			.user(TEST_USER, TEST_PASSWORD, false)
			.build()
			.await;

		ctx.playlist_manager
			.save_playlist(TEST_PLAYLIST_NAME, TEST_USER, Vec::new())
			.await
			.unwrap();

		let found_playlists = ctx
			.playlist_manager
			.list_playlists(TEST_USER)
			.await
			.unwrap();

		assert_eq!(found_playlists.len(), 1);
		assert_eq!(found_playlists[0].name.as_str(), TEST_PLAYLIST_NAME);
	}

	#[tokio::test]
	async fn save_playlist_is_idempotent() {
		let ctx = test::ContextBuilder::new(test_name!())
			.user(TEST_USER, TEST_PASSWORD, false)
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build()
			.await;

		ctx.scanner.run_scan().await.unwrap();

		let songs = list_all_songs(&ctx).await;

		ctx.playlist_manager
			.save_playlist(TEST_PLAYLIST_NAME, TEST_USER, songs.clone())
			.await
			.unwrap();

		ctx.playlist_manager
			.save_playlist(TEST_PLAYLIST_NAME, TEST_USER, songs.clone())
			.await
			.unwrap();

		let playlist = ctx
			.playlist_manager
			.read_playlist(TEST_PLAYLIST_NAME, TEST_USER)
			.await
			.unwrap();
		assert_eq!(playlist.songs.len(), 13);
	}

	#[tokio::test]
	async fn delete_playlist_golden_path() {
		let ctx = test::ContextBuilder::new(test_name!())
			.user(TEST_USER, TEST_PASSWORD, false)
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build()
			.await;

		ctx.scanner.run_scan().await.unwrap();
		let songs = list_all_songs(&ctx).await;

		ctx.playlist_manager
			.save_playlist(TEST_PLAYLIST_NAME, TEST_USER, songs)
			.await
			.unwrap();

		ctx.playlist_manager
			.delete_playlist(TEST_PLAYLIST_NAME, TEST_USER)
			.await
			.unwrap();

		let found_playlists = ctx
			.playlist_manager
			.list_playlists(TEST_USER)
			.await
			.unwrap();
		assert_eq!(found_playlists.len(), 0);
	}

	#[tokio::test]
	async fn read_playlist_golden_path() {
		let ctx = test::ContextBuilder::new(test_name!())
			.user(TEST_USER, TEST_PASSWORD, false)
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build()
			.await;

		ctx.scanner.run_scan().await.unwrap();

		let songs = list_all_songs(&ctx).await;

		ctx.playlist_manager
			.save_playlist(TEST_PLAYLIST_NAME, TEST_USER, songs)
			.await
			.unwrap();

		let playlist = ctx
			.playlist_manager
			.read_playlist(TEST_PLAYLIST_NAME, TEST_USER)
			.await
			.unwrap();

		assert_eq!(playlist.songs.len(), 13);

		let first_song_path: PathBuf = [
			TEST_MOUNT_NAME,
			"Khemmis",
			"Hunted",
			"01 - Above The Water.mp3",
		]
		.iter()
		.collect();
		assert_eq!(playlist.songs[0], first_song_path);
	}

	#[tokio::test]
	async fn playlists_are_sorted_alphabetically() {
		let ctx = test::ContextBuilder::new(test_name!())
			.user(TEST_USER, TEST_PASSWORD, false)
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build()
			.await;

		for name in ["ax", "b", "Ay", "B", "àz"] {
			ctx.playlist_manager
				.save_playlist(name, TEST_USER, Vec::new())
				.await
				.unwrap();
		}

		let playlists = ctx
			.playlist_manager
			.list_playlists(TEST_USER)
			.await
			.unwrap();

		let names = playlists
			.into_iter()
			.map(|p| p.name.to_string())
			.collect::<Vec<_>>();

		assert_eq!(names, vec!["ax", "Ay", "àz", "B", "b"]);
	}

	#[tokio::test]
	async fn export_import_playlist_zip() {
		let ctx = test::ContextBuilder::new(test_name!())
			.user(TEST_USER, TEST_PASSWORD, false)
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build()
			.await;

		ctx.scanner.run_scan().await.unwrap();

		let songs = list_all_songs(&ctx).await;

		ctx.playlist_manager
			.save_playlist(TEST_PLAYLIST_NAME, TEST_USER, songs)
			.await
			.unwrap();

		let zip_data = ctx
			.playlist_manager
			.export_playlists(TEST_USER)
			.await
			.unwrap();

		ctx.playlist_manager
			.delete_playlist(TEST_PLAYLIST_NAME, TEST_USER)
			.await
			.unwrap();

		let playlists = ctx.playlist_manager.list_playlists(TEST_USER).await;
		assert!(playlists.unwrap().is_empty());

		let payload = HashMap::from_iter([("archive.zip".to_owned(), zip_data)]);
		ctx.playlist_manager
			.import_playlists(TEST_USER, payload)
			.await
			.unwrap();

		let playlist = ctx
			.playlist_manager
			.read_playlist(TEST_PLAYLIST_NAME, TEST_USER)
			.await
			.unwrap();

		assert_eq!(playlist.songs.len(), 13);

		let first_song_path: PathBuf = [
			TEST_MOUNT_NAME,
			"Khemmis",
			"Hunted",
			"01 - Above The Water.mp3",
		]
		.iter()
		.collect();
		assert_eq!(playlist.songs[0], first_song_path);
	}
}
