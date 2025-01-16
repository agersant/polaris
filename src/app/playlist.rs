use core::clone::Clone;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::time::Duration;

use icu_collator::{Collator, CollatorOptions, Strength};
use native_db::*;
use native_model::{native_model, Model};
use serde::{Deserialize, Serialize};
use tokio::task::spawn_blocking;

use crate::app::{index, ndb, Error};

#[derive(Clone)]
pub struct Manager {
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
	pub fn new(db: ndb::Manager) -> Self {
		Self { db }
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
	}

	pub async fn save_playlist(
		&self,
		name: &str,
		owner: &str,
		songs: Vec<index::Song>,
	) -> Result<(), Error> {
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
	}

	pub async fn read_playlist(&self, name: &str, owner: &str) -> Result<Playlist, Error> {
		spawn_blocking({
			let manager = self.clone();
			let owner = owner.to_owned();
			let name = name.to_owned();
			move || {
				let transaction = manager.db.r_transaction()?;
				match transaction.get().primary::<PlaylistModel>((owner, name)) {
					Ok(Some(p)) => Ok(Playlist::from(p)),
					Ok(None) => Err(Error::PlaylistNotFound),
					Err(e) => Err(Error::NativeDatabase(e)),
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
				let transaction = manager.db.rw_transaction()?;
				let playlist = match transaction
					.get()
					.primary::<PlaylistModel>((owner.as_str(), name.as_str()))
				{
					Ok(Some(p)) => Ok(p),
					Ok(None) => Err(Error::PlaylistNotFound),
					Err(e) => Err(Error::NativeDatabase(e)),
				}?;
				transaction.remove::<PlaylistModel>(playlist)?;
				transaction.commit()?;
				Ok(())
			}
		})
		.await?
	}
}

#[cfg(test)]
mod test {
	use std::path::PathBuf;

	use crate::app::index;
	use crate::app::test::{self, Context};
	use crate::test_name;

	const TEST_USER: &str = "test_user";
	const TEST_PASSWORD: &str = "password";
	const TEST_PLAYLIST_NAME: &str = "Chill & Grill";
	const TEST_MOUNT_NAME: &str = "root";

	async fn list_all_songs(ctx: &Context) -> Vec<index::Song> {
		let paths = ctx
			.index_manager
			.flatten(PathBuf::from(TEST_MOUNT_NAME))
			.await
			.unwrap()
			.into_iter()
			.collect::<Vec<_>>();

		let songs = ctx
			.index_manager
			.get_songs(paths)
			.await
			.into_iter()
			.map(|s| s.unwrap())
			.collect::<Vec<_>>();

		assert_eq!(songs.len(), 13);
		songs
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
}
