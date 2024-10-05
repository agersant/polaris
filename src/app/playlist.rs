use core::clone::Clone;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use sqlx::{Acquire, QueryBuilder, Sqlite};

use crate::app::Error;
use crate::db::DB;

#[derive(Clone)]
pub struct Manager {
	db: DB,
}

#[derive(Debug)]
pub struct PlaylistHeader {
	pub name: String,
	pub duration: Duration,
	pub num_songs_by_genre: HashMap<String, u32>,
}

impl Manager {
	pub fn new(db: DB) -> Self {
		Self { db }
	}

	pub async fn list_playlists(&self, owner: &str) -> Result<Vec<PlaylistHeader>, Error> {
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
		struct PlaylistSong {
			virtual_path: String,
			ordering: i64,
		}

		let mut new_songs: Vec<PlaylistSong> = Vec::with_capacity(content.len());
		for (i, virtual_path) in content.iter().enumerate() {
			new_songs.push(PlaylistSong {
				virtual_path: virtual_path.to_string_lossy().to_string(),
				ordering: i as i64,
			});
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
			QueryBuilder::<Sqlite>::new(
				"INSERT INTO playlist_songs (playlist, virtual_path, ordering) ",
			)
			.push_values(chunk, |mut b, song| {
				b.push_bind(playlist_id)
					.push_bind(&song.virtual_path)
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
	) -> Result<Vec<PathBuf>, Error> {
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
			sqlx::query_scalar!(
				r#"
					SELECT virtual_path
					FROM playlist_songs ps
					WHERE ps.playlist = $1
					ORDER BY ps.ordering
				"#,
				playlist_id
			)
			.fetch_all(connection.as_mut())
			.await?
			.into_iter()
			.map(PathBuf::from)
			.collect()
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
	use std::path::PathBuf;

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
		assert_eq!(found_playlists[0].name, TEST_PLAYLIST_NAME);
	}

	#[tokio::test]
	async fn save_playlist_is_idempotent() {
		let mut ctx = test::ContextBuilder::new(test_name!())
			.user(TEST_USER, TEST_PASSWORD, false)
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build()
			.await;

		ctx.scanner.update().await.unwrap();

		let playlist_content = ctx
			.index_manager
			.flatten(PathBuf::from(TEST_MOUNT_NAME))
			.await
			.unwrap()
			.into_iter()
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

		ctx.scanner.update().await.unwrap();

		let playlist_content = ctx
			.index_manager
			.flatten(PathBuf::from(TEST_MOUNT_NAME))
			.await
			.unwrap()
			.into_iter()
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
		assert_eq!(songs[0], first_song_path);
	}
}
