use core::clone::Clone;
use sqlx::{Acquire, QueryBuilder, Sqlite};
use std::path::PathBuf;

use crate::app::collection::SongKey;
use crate::app::vfs;
use crate::db::{self, DB};

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error(transparent)]
	Database(#[from] sqlx::Error),
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

	pub async fn list_playlists(&self, owner: &str) -> Result<Vec<String>, Error> {
		let mut connection = self.db.connect().await?;

		let user_id = sqlx::query_scalar!("SELECT id FROM users WHERE name = $1", owner)
			.fetch_optional(connection.as_mut())
			.await?
			.ok_or(Error::UserNotFound)?;

		Ok(
			sqlx::query_scalar!("SELECT name FROM playlists WHERE owner = $1", user_id)
				.fetch_all(connection.as_mut())
				.await?,
		)
	}

	pub async fn save_playlist(
		&self,
		playlist_name: &str,
		owner: &str,
		content: &[PathBuf],
	) -> Result<(), Error> {
		let vfs = self.vfs_manager.get_vfs().await?;

		struct PlaylistSong {
			path: String,
			ordering: i64,
		}

		let mut new_songs: Vec<PlaylistSong> = Vec::with_capacity(content.len());
		for (i, path) in content.iter().enumerate() {
			if let Some(real_path) = vfs
				.virtual_to_real(path)
				.ok()
				.and_then(|p| p.to_str().map(|s| s.to_owned()))
			{
				new_songs.push(PlaylistSong {
					path: real_path,
					ordering: i as i64,
				});
			}
		}

		let mut connection = self.db.connect().await?;

		// Find owner
		let user_id = sqlx::query_scalar!("SELECT id FROM users WHERE name = $1", owner)
			.fetch_optional(connection.as_mut())
			.await?
			.ok_or(Error::UserNotFound)?;

		// Create playlist
		sqlx::query!(
			"INSERT INTO playlists (owner, name) VALUES($1, $2)",
			user_id,
			playlist_name
		)
		.execute(connection.as_mut())
		.await?;

		let playlist_id = sqlx::query_scalar!(
			"SELECT id FROM playlists WHERE owner = $1 AND name = $2",
			user_id,
			playlist_name
		)
		.fetch_one(connection.as_mut())
		.await?;

		connection.acquire().await?;

		sqlx::query!(
			"DELETE FROM playlist_songs WHERE playlist = $1",
			playlist_id
		)
		.execute(connection.as_mut())
		.await?;

		for chunk in new_songs.chunks(10_000) {
			QueryBuilder::<Sqlite>::new("INSERT INTO playlist_songs (playlist, path, ordering) ")
				.push_values(chunk, |mut b, song| {
					b.push_bind(playlist_id)
						.push_bind(&song.path)
						.push_bind(song.ordering);
				})
				.build()
				.execute(connection.as_mut())
				.await?;
		}

		Ok(())
	}

	pub async fn read_playlist(
		&self,
		playlist_name: &str,
		owner: &str,
	) -> Result<Vec<SongKey>, Error> {
		let songs = {
			let mut connection = self.db.connect().await?;

			// Find owner
			let user_id = sqlx::query_scalar!("SELECT id FROM users WHERE name = $1", owner)
				.fetch_optional(connection.as_mut())
				.await?
				.ok_or(Error::UserNotFound)?;

			// Find playlist
			let playlist_id = sqlx::query_scalar!(
				"SELECT id FROM playlists WHERE name = $1 and owner = $2",
				playlist_name,
				user_id
			)
			.fetch_optional(connection.as_mut())
			.await?
			.ok_or(Error::PlaylistNotFound)?;

			// List songs
			todo!();
			// sqlx::query_as!(
			// 	Song,
			// 	r#"
			// 		SELECT s.*
			// 		FROM playlist_songs ps
			// 		INNER JOIN songs s ON ps.virtual_path = s.virtual_path
			// 		WHERE ps.playlist = $1
			// 		ORDER BY ps.ordering
			// 	"#,
			// 	playlist_id
			// )
			// .fetch_all(connection.as_mut())
			// .await?
		};

		Ok(songs)
	}

	pub async fn delete_playlist(&self, playlist_name: &str, owner: &str) -> Result<(), Error> {
		let mut connection = self.db.connect().await?;

		let user_id = sqlx::query_scalar!("SELECT id FROM users WHERE name = $1", owner)
			.fetch_optional(connection.as_mut())
			.await?
			.ok_or(Error::UserNotFound)?;

		let num_deletions = sqlx::query_scalar!(
			"DELETE FROM playlists WHERE owner = $1 AND name = $2",
			user_id,
			playlist_name
		)
		.execute(connection.as_mut())
		.await?
		.rows_affected();

		match num_deletions {
			0 => Err(Error::PlaylistNotFound),
			_ => Ok(()),
		}
	}
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

	#[tokio::test]
	async fn save_playlist_golden_path() {
		let ctx = test::ContextBuilder::new(test_name!())
			.user(TEST_USER, TEST_PASSWORD, false)
			.build()
			.await;

		ctx.playlist_manager
			.save_playlist(TEST_PLAYLIST_NAME, TEST_USER, &Vec::new())
			.await
			.unwrap();

		let found_playlists = ctx
			.playlist_manager
			.list_playlists(TEST_USER)
			.await
			.unwrap();
		assert_eq!(found_playlists.len(), 1);
		assert_eq!(found_playlists[0], TEST_PLAYLIST_NAME);
	}

	#[tokio::test]
	async fn save_playlist_is_idempotent() {
		let mut ctx = test::ContextBuilder::new(test_name!())
			.user(TEST_USER, TEST_PASSWORD, false)
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build()
			.await;

		ctx.updater.update().await.unwrap();

		let playlist_content = ctx
			.browser
			.flatten(Path::new(TEST_MOUNT_NAME))
			.await
			.unwrap()
			.into_iter()
			.map(|s| s.virtual_path)
			.collect::<Vec<_>>();
		assert_eq!(playlist_content.len(), 13);

		ctx.playlist_manager
			.save_playlist(TEST_PLAYLIST_NAME, TEST_USER, &playlist_content)
			.await
			.unwrap();

		ctx.playlist_manager
			.save_playlist(TEST_PLAYLIST_NAME, TEST_USER, &playlist_content)
			.await
			.unwrap();

		let songs = ctx
			.playlist_manager
			.read_playlist(TEST_PLAYLIST_NAME, TEST_USER)
			.await
			.unwrap();
		assert_eq!(songs.len(), 13);
	}

	#[tokio::test]
	async fn delete_playlist_golden_path() {
		let ctx = test::ContextBuilder::new(test_name!())
			.user(TEST_USER, TEST_PASSWORD, false)
			.build()
			.await;

		let playlist_content = Vec::new();

		ctx.playlist_manager
			.save_playlist(TEST_PLAYLIST_NAME, TEST_USER, &playlist_content)
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
		let mut ctx = test::ContextBuilder::new(test_name!())
			.user(TEST_USER, TEST_PASSWORD, false)
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build()
			.await;

		ctx.updater.update().await.unwrap();

		let playlist_content = ctx
			.browser
			.flatten(Path::new(TEST_MOUNT_NAME))
			.await
			.unwrap()
			.into_iter()
			.map(|s| s.virtual_path)
			.collect::<Vec<_>>();
		assert_eq!(playlist_content.len(), 13);

		ctx.playlist_manager
			.save_playlist(TEST_PLAYLIST_NAME, TEST_USER, &playlist_content)
			.await
			.unwrap();

		let songs = ctx
			.playlist_manager
			.read_playlist(TEST_PLAYLIST_NAME, TEST_USER)
			.await
			.unwrap();

		assert_eq!(songs.len(), 13);

		let first_song_path: PathBuf = [
			TEST_MOUNT_NAME,
			"Khemmis",
			"Hunted",
			"01 - Above The Water.mp3",
		]
		.iter()
		.collect();
		assert_eq!(songs[0].virtual_path, first_song_path);
	}
}
