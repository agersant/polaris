use core::clone::Clone;
use diesel::prelude::*;
use diesel::BelongingToDsl;
use std::path::Path;

use crate::app::index::Song;
use crate::app::vfs;
use crate::db::{self, playlist_songs, playlists, songs, users, DB};
use crate::service::dto;

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error(transparent)]
	Database(#[from] diesel::result::Error),
	#[error(transparent)]
	DatabaseConnection(#[from] db::Error),
	#[error("User not found")]
	UserNotFound,
	#[error("Playlist not found")]
	PlaylistNotFound,
	#[error(transparent)]
	Vfs(#[from] vfs::Error),
}

#[derive(Clone)]
pub struct Manager {
	db: DB,
	vfs_manager: vfs::Manager,
}

impl Manager {
	pub fn new(db: DB, vfs_manager: vfs::Manager) -> Self {
		Self { db, vfs_manager }
	}

	pub fn list_playlists(&self, owner: &str) -> Result<Vec<String>, Error> {
		let mut connection = self.db.connect()?;

		let user: User = {
			use self::users::dsl::*;
			users
				.filter(name.eq(owner))
				.select((id,))
				.first(&mut connection)
				.optional()?
				.ok_or(Error::UserNotFound)?
		};

		{
			use self::playlists::dsl::*;
			let found_playlists: Vec<String> = Playlist::belonging_to(&user)
				.select(name)
				.load(&mut connection)?;
			Ok(found_playlists)
		}
	}

	pub fn save_playlist(
		&self,
		playlist_name: &str,
		owner: &str,
		content: &[String],
	) -> Result<(), Error> {
		let new_playlist: NewPlaylist;
		let playlist: Playlist;
		let vfs = self.vfs_manager.get_vfs()?;

		{
			let mut connection = self.db.connect()?;

			// Find owner
			let user: User = {
				use self::users::dsl::*;
				users
					.filter(name.eq(owner))
					.select((id,))
					.first(&mut connection)
					.optional()?
					.ok_or(Error::UserNotFound)?
			};

			// Create playlist
			new_playlist = NewPlaylist {
				name: playlist_name.into(),
				owner: user.id,
			};

			diesel::insert_into(playlists::table)
				.values(&new_playlist)
				.execute(&mut connection)?;

			playlist = {
				use self::playlists::dsl::*;
				playlists
					.select((id, owner))
					.filter(name.eq(playlist_name).and(owner.eq(user.id)))
					.get_result(&mut connection)?
			}
		}

		let mut new_songs: Vec<NewPlaylistSong> = Vec::new();
		new_songs.reserve(content.len());

		for (i, path) in content.iter().enumerate() {
			let virtual_path = Path::new(&path);
			if let Some(real_path) = vfs
				.virtual_to_real(virtual_path)
				.ok()
				.and_then(|p| p.to_str().map(|s| s.to_owned()))
			{
				new_songs.push(NewPlaylistSong {
					playlist: playlist.id,
					path: real_path,
					ordering: i as i32,
				});
			}
		}

		{
			let mut connection = self.db.connect()?;
			connection.transaction::<_, diesel::result::Error, _>(|connection| {
				// Delete old content (if any)
				let old_songs = PlaylistSong::belonging_to(&playlist);
				diesel::delete(old_songs).execute(connection)?;

				// Insert content
				diesel::insert_into(playlist_songs::table)
					.values(&new_songs)
					.execute(&mut *connection)?; // TODO https://github.com/diesel-rs/diesel/issues/1822
				Ok(())
			})?;
		}

		Ok(())
	}

	pub fn read_playlist(&self, playlist_name: &str, owner: &str) -> Result<Vec<dto::Song>, Error> {
		let vfs = self.vfs_manager.get_vfs()?;

		let mut connection = self.db.connect()?;

		// Find owner
		let user: User = {
			use self::users::dsl::*;
			users
				.filter(name.eq(owner))
				.select((id,))
				.first(&mut connection)
				.optional()?
				.ok_or(Error::UserNotFound)?
		};

		// Find playlist
		let playlist: Playlist = {
			use self::playlists::dsl::*;
			playlists
				.select((id, owner))
				.filter(name.eq(playlist_name).and(owner.eq(user.id)))
				.get_result(&mut connection)
				.optional()?
				.ok_or(Error::PlaylistNotFound)?
		};

		let songs: Vec<Song> = {
			playlist_songs::table
				.filter(playlist_songs::playlist.eq(playlist.id))
				.inner_join(songs::table.on(playlist_songs::path.eq(songs::path)))
				.select(songs::all_columns)
				.get_results(&mut connection)?
		};

		// Map real path to virtual paths
		Ok(songs
			.into_iter()
			.filter_map(|s| s.virtualize(&vfs))
			.map(|s| s.fetch_artists(&mut connection))
			.collect::<Result<_, _>>()?)
	}

	pub fn delete_playlist(&self, playlist_name: &str, owner: &str) -> Result<(), Error> {
		let mut connection = self.db.connect()?;

		let user: User = {
			use self::users::dsl::*;
			users
				.filter(name.eq(owner))
				.select((id,))
				.first(&mut connection)
				.optional()?
				.ok_or(Error::UserNotFound)?
		};

		{
			use self::playlists::dsl::*;
			let q = Playlist::belonging_to(&user).filter(name.eq(playlist_name));
			match diesel::delete(q).execute(&mut connection)? {
				0 => Err(Error::PlaylistNotFound),
				_ => Ok(()),
			}
		}
	}
}

#[derive(Identifiable, Queryable, Associations)]
#[diesel(belongs_to(User, foreign_key = owner))]
struct Playlist {
	id: i32,
	owner: i32,
}

#[derive(Identifiable, Queryable, Associations)]
#[diesel(belongs_to(Playlist, foreign_key = playlist))]
struct PlaylistSong {
	id: i32,
	playlist: i32,
}

#[derive(Insertable)]
#[diesel(table_name = playlists)]
struct NewPlaylist {
	name: String,
	owner: i32,
}

#[derive(Insertable)]
#[diesel(table_name = playlist_songs)]
struct NewPlaylistSong {
	playlist: i32,
	path: String,
	ordering: i32,
}

#[derive(Identifiable, Queryable)]
struct User {
	id: i32,
}

#[cfg(test)]
mod test {
	use std::path::{Path, PathBuf};

	use crate::app::test;
	use crate::test_name;

	const TEST_USER: &str = "test_user";
	const TEST_PASSWORD: &str = "password";
	const TEST_PLAYLIST_NAME: &str = "Chill & Grill";
	const TEST_MOUNT_NAME: &str = "root";

	#[test]
	fn save_playlist_golden_path() {
		let ctx = test::ContextBuilder::new(test_name!())
			.user(TEST_USER, TEST_PASSWORD, false)
			.build();

		ctx.playlist_manager
			.save_playlist(TEST_PLAYLIST_NAME, TEST_USER, &Vec::new())
			.unwrap();

		let found_playlists = ctx.playlist_manager.list_playlists(TEST_USER).unwrap();
		assert_eq!(found_playlists.len(), 1);
		assert_eq!(found_playlists[0], TEST_PLAYLIST_NAME);
	}

	#[test]
	fn save_playlist_is_idempotent() {
		let ctx = test::ContextBuilder::new(test_name!())
			.user(TEST_USER, TEST_PASSWORD, false)
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build();

		ctx.index.update().unwrap();

		let playlist_content: Vec<String> = ctx
			.index
			.flatten(Path::new(TEST_MOUNT_NAME))
			.unwrap()
			.into_iter()
			.map(|s| s.path)
			.collect();
		assert_eq!(playlist_content.len(), 13);

		ctx.playlist_manager
			.save_playlist(TEST_PLAYLIST_NAME, TEST_USER, &playlist_content)
			.unwrap();

		ctx.playlist_manager
			.save_playlist(TEST_PLAYLIST_NAME, TEST_USER, &playlist_content)
			.unwrap();

		let songs = ctx
			.playlist_manager
			.read_playlist(TEST_PLAYLIST_NAME, TEST_USER)
			.unwrap();
		assert_eq!(songs.len(), 13);
	}

	#[test]
	fn delete_playlist_golden_path() {
		let ctx = test::ContextBuilder::new(test_name!())
			.user(TEST_USER, TEST_PASSWORD, false)
			.build();

		let playlist_content = Vec::new();

		ctx.playlist_manager
			.save_playlist(TEST_PLAYLIST_NAME, TEST_USER, &playlist_content)
			.unwrap();

		ctx.playlist_manager
			.delete_playlist(TEST_PLAYLIST_NAME, TEST_USER)
			.unwrap();

		let found_playlists = ctx.playlist_manager.list_playlists(TEST_USER).unwrap();
		assert_eq!(found_playlists.len(), 0);
	}

	#[test]
	fn read_playlist_golden_path() {
		let ctx = test::ContextBuilder::new(test_name!())
			.user(TEST_USER, TEST_PASSWORD, false)
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build();

		ctx.index.update().unwrap();

		let playlist_content: Vec<String> = ctx
			.index
			.flatten(Path::new(TEST_MOUNT_NAME))
			.unwrap()
			.into_iter()
			.map(|s| s.path)
			.collect();
		assert_eq!(playlist_content.len(), 13);

		ctx.playlist_manager
			.save_playlist(TEST_PLAYLIST_NAME, TEST_USER, &playlist_content)
			.unwrap();

		let songs = ctx
			.playlist_manager
			.read_playlist(TEST_PLAYLIST_NAME, TEST_USER)
			.unwrap();

		assert_eq!(songs.len(), 13);
		assert_eq!(songs[0].title, Some("Above The Water".to_owned()));

		let first_song_path: PathBuf = [
			TEST_MOUNT_NAME,
			"Khemmis",
			"Hunted",
			"01 - Above The Water.mp3",
		]
		.iter()
		.collect();
		assert_eq!(songs[0].path, first_song_path.to_str().unwrap());
	}
}
