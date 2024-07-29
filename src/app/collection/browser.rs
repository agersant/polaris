use std::path::Path;

use crate::app::{collection, vfs};
use crate::db::DB;

#[derive(Clone)]
pub struct Browser {
	db: DB,
	vfs_manager: vfs::Manager,
}

impl Browser {
	pub fn new(db: DB, vfs_manager: vfs::Manager) -> Self {
		Self { db, vfs_manager }
	}

	pub async fn browse<P>(&self, path: P) -> Result<Vec<collection::File>, collection::Error>
	where
		P: AsRef<Path>,
	{
		let mut output = Vec::new();
		let mut connection = self.db.connect().await?;

		if path.as_ref().components().count() == 0 {
			// Browse top-level
			let directories = sqlx::query_as!(
				collection::Directory,
				"SELECT * FROM directories WHERE virtual_parent IS NULL"
			)
			.fetch_all(connection.as_mut())
			.await?;
			output.extend(directories.into_iter().map(collection::File::Directory));
		} else {
			let vfs = self.vfs_manager.get_vfs().await?;
			match vfs.virtual_to_real(&path) {
				Ok(p) if p.exists() => {}
				_ => {
					return Err(collection::Error::DirectoryNotFound(
						path.as_ref().to_owned(),
					))
				}
			}

			let path = path.as_ref().to_string_lossy();

			// Browse sub-directory
			let directories = sqlx::query_as!(
				collection::Directory,
				"SELECT * FROM directories WHERE virtual_parent = $1 ORDER BY virtual_path COLLATE NOCASE ASC",
				path
			)
			.fetch_all(connection.as_mut())
			.await?;
			output.extend(directories.into_iter().map(collection::File::Directory));

			let songs = sqlx::query_as!(
				collection::Song,
				"SELECT * FROM songs WHERE virtual_parent = $1 ORDER BY virtual_path COLLATE NOCASE ASC",
				path
			)
			.fetch_all(connection.as_mut())
			.await?;

			output.extend(songs.into_iter().map(collection::File::Song));
		}

		Ok(output)
	}

	pub async fn flatten<P>(&self, path: P) -> Result<Vec<collection::Song>, collection::Error>
	where
		P: AsRef<Path>,
	{
		let mut connection = self.db.connect().await?;

		let songs = if path.as_ref().parent().is_some() {
			let vfs = self.vfs_manager.get_vfs().await?;
			match vfs.virtual_to_real(&path) {
				Ok(p) if p.exists() => {}
				_ => {
					return Err(collection::Error::DirectoryNotFound(
						path.as_ref().to_owned(),
					))
				}
			}

			let song_path_filter = {
				let mut path_buf = path.as_ref().to_owned();
				path_buf.push("%");
				path_buf.as_path().to_string_lossy().into_owned()
			};
			sqlx::query_as!(
				collection::Song,
				"SELECT * FROM songs WHERE virtual_path LIKE $1 ORDER BY virtual_path COLLATE NOCASE ASC",
				song_path_filter
			)
			.fetch_all(connection.as_mut())
			.await?
		} else {
			sqlx::query_as!(
				collection::Song,
				"SELECT * FROM songs ORDER BY virtual_path COLLATE NOCASE ASC"
			)
			.fetch_all(connection.as_mut())
			.await?
		};

		Ok(songs)
	}

	pub async fn search(&self, query: &str) -> Result<Vec<collection::File>, collection::Error> {
		let mut connection = self.db.connect().await?;
		let like_test = format!("%{}%", query);
		let mut output = Vec::new();

		// Find dirs with matching path and parent not matching
		{
			let directories = sqlx::query_as!(
				collection::Directory,
				"SELECT * FROM directories WHERE virtual_path LIKE $1 AND virtual_parent NOT LIKE $1",
				like_test
			)
			.fetch_all(connection.as_mut())
			.await?;

			output.extend(directories.into_iter().map(collection::File::Directory));
		}

		// Find songs with matching title/album/artist and non-matching parent
		{
			let songs = sqlx::query_as!(
				collection::Song,
				r#"
				SELECT * FROM songs
				WHERE	(	virtual_path LIKE $1
						OR  title LIKE $1
						OR  album LIKE $1
						OR  artists LIKE $1
						OR  album_artists LIKE $1
						)
					AND virtual_parent NOT LIKE $1
				"#,
				like_test
			)
			.fetch_all(connection.as_mut())
			.await?;

			output.extend(songs.into_iter().map(collection::File::Song));
		}

		Ok(output)
	}

	pub async fn get_song(&self, path: &Path) -> Result<collection::Song, collection::Error> {
		let mut connection = self.db.connect().await?;

		let path = path.to_string_lossy();
		let song = sqlx::query_as!(
			collection::Song,
			"SELECT * FROM songs WHERE virtual_path = $1",
			path
		)
		.fetch_one(connection.as_mut())
		.await?;

		Ok(song)
	}
}

#[cfg(test)]
mod test {
	use std::path::{Path, PathBuf};

	use super::*;
	use crate::app::test;
	use crate::test_name;

	const TEST_MOUNT_NAME: &str = "root";

	#[tokio::test]
	async fn can_browse_top_level() {
		let mut ctx = test::ContextBuilder::new(test_name!())
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build()
			.await;
		ctx.updater.update().await.unwrap();

		let root_path = Path::new(TEST_MOUNT_NAME);
		let files = ctx.browser.browse(Path::new("")).await.unwrap();
		assert_eq!(files.len(), 1);
		match files[0] {
			collection::File::Directory(ref d) => {
				assert_eq!(d.virtual_path, root_path.to_str().unwrap())
			}
			_ => panic!("Expected directory"),
		}
	}

	#[tokio::test]
	async fn can_browse_directory() {
		let khemmis_path: PathBuf = [TEST_MOUNT_NAME, "Khemmis"].iter().collect();
		let tobokegao_path: PathBuf = [TEST_MOUNT_NAME, "Tobokegao"].iter().collect();

		let mut ctx = test::ContextBuilder::new(test_name!())
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build()
			.await;
		ctx.updater.update().await.unwrap();

		let files = ctx
			.browser
			.browse(Path::new(TEST_MOUNT_NAME))
			.await
			.unwrap();

		assert_eq!(files.len(), 2);
		match files[0] {
			collection::File::Directory(ref d) => {
				assert_eq!(d.virtual_path, khemmis_path.to_str().unwrap())
			}
			_ => panic!("Expected directory"),
		}

		match files[1] {
			collection::File::Directory(ref d) => {
				assert_eq!(d.virtual_path, tobokegao_path.to_str().unwrap())
			}
			_ => panic!("Expected directory"),
		}
	}

	#[tokio::test]
	async fn can_flatten_root() {
		let mut ctx = test::ContextBuilder::new(test_name!())
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build()
			.await;
		ctx.updater.update().await.unwrap();
		let songs = ctx
			.browser
			.flatten(Path::new(TEST_MOUNT_NAME))
			.await
			.unwrap();
		assert_eq!(songs.len(), 13);
		assert_eq!(songs[0].title, Some("Above The Water".to_owned()));
	}

	#[tokio::test]
	async fn can_flatten_directory() {
		let mut ctx = test::ContextBuilder::new(test_name!())
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build()
			.await;
		ctx.updater.update().await.unwrap();
		let path: PathBuf = [TEST_MOUNT_NAME, "Tobokegao"].iter().collect();
		let songs = ctx.browser.flatten(path).await.unwrap();
		assert_eq!(songs.len(), 8);
	}

	#[tokio::test]
	async fn can_flatten_directory_with_shared_prefix() {
		let mut ctx = test::ContextBuilder::new(test_name!())
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build()
			.await;
		ctx.updater.update().await.unwrap();
		let path: PathBuf = [TEST_MOUNT_NAME, "Tobokegao", "Picnic"].iter().collect(); // Prefix of '(Picnic Remixes)'
		let songs = ctx.browser.flatten(path).await.unwrap();
		assert_eq!(songs.len(), 7);
	}

	#[tokio::test]
	async fn can_get_a_song() {
		let mut ctx = test::ContextBuilder::new(test_name!())
			.mount(TEST_MOUNT_NAME, "test-data/small-collection")
			.build()
			.await;

		ctx.updater.update().await.unwrap();

		let picnic_virtual_dir: PathBuf = [TEST_MOUNT_NAME, "Tobokegao", "Picnic"].iter().collect();
		let song_virtual_path = picnic_virtual_dir.join("05 - シャーベット (Sherbet).mp3");
		let artwork_virtual_path = picnic_virtual_dir.join("Folder.png");

		let song = ctx.browser.get_song(&song_virtual_path).await.unwrap();
		assert_eq!(
			song.virtual_path,
			song_virtual_path.to_string_lossy().as_ref()
		);
		assert_eq!(song.track_number, Some(5));
		assert_eq!(song.disc_number, None);
		assert_eq!(song.title, Some("シャーベット (Sherbet)".to_owned()));
		assert_eq!(
			song.artists,
			collection::MultiString(vec!["Tobokegao".to_owned()])
		);
		assert_eq!(song.album_artists, collection::MultiString(vec![]));
		assert_eq!(song.album, Some("Picnic".to_owned()));
		assert_eq!(song.year, Some(2016));
		assert_eq!(
			song.artwork,
			Some(artwork_virtual_path.to_string_lossy().into_owned())
		);
	}
}
