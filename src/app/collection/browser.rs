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
		todo!();
	}

	pub async fn flatten<P>(&self, path: P) -> Result<Vec<collection::Song>, collection::Error>
	where
		P: AsRef<Path>,
	{
		todo!();
	}

	pub async fn search(&self, query: &str) -> Result<Vec<collection::File>, collection::Error> {
		todo!();
	}

	pub async fn get_song(&self, path: &Path) -> Result<collection::Song, collection::Error> {
		todo!();
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
				assert_eq!(d, &root_path)
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
				assert_eq!(d, &khemmis_path)
			}
			_ => panic!("Expected directory"),
		}

		match files[1] {
			collection::File::Directory(ref d) => {
				assert_eq!(d, &tobokegao_path)
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
		assert_eq!(song.virtual_path, song_virtual_path);
		assert_eq!(song.track_number, Some(5));
		assert_eq!(song.disc_number, None);
		assert_eq!(song.title, Some("シャーベット (Sherbet)".to_owned()));
		assert_eq!(song.artists, vec!["Tobokegao".to_owned()]);
		assert_eq!(song.album_artists, Vec::<String>::new());
		assert_eq!(song.album, Some("Picnic".to_owned()));
		assert_eq!(song.year, Some(2016));
		assert_eq!(song.artwork, Some(artwork_virtual_path));
	}
}
