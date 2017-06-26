use core::ops::Deref;
use diesel;
use diesel::expression::sql;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel::types;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use db::schema::{directories, songs};
use errors::*;
use vfs::Vfs;

mod index;
mod models;
mod schema;

pub use self::index::{Index, IndexConfig};
pub use self::models::{CollectionFile, Directory, Song};

#[allow(dead_code)]
const DB_MIGRATIONS_PATH: &'static str = "src/db/migrations";
embed_migrations!("src/db/migrations");

pub struct DB {
	vfs: Arc<Vfs>,
	connection: Arc<Mutex<SqliteConnection>>,
	index: Index,
}

impl DB {
	pub fn new(vfs: Arc<Vfs>, config: &IndexConfig) -> Result<DB> {
		let path = &config.path;
		println!("Index file path: {}", path.to_string_lossy());
		let connection =
			Arc::new(Mutex::new(SqliteConnection::establish(&path.to_string_lossy())?));
		let db = DB {
			vfs: vfs.clone(),
			connection: connection.clone(),
			index: Index::new(vfs, connection.clone(), config),
		};
		db.init()?;
		Ok(db)
	}

	fn init(&self) -> Result<()> {
		{
			let connection = self.connection.lock().unwrap();
			connection.execute("PRAGMA synchronous = NORMAL")?;
		}
		self.migrate_up()?;
		Ok(())
	}

	pub fn get_index(&self) -> &Index {
		&self.index
	}

	#[allow(dead_code)]
	fn migrate_down(&self) -> Result<()> {
		let connection = self.connection.lock().unwrap();
		let connection = connection.deref();
		loop {
			match diesel::migrations::revert_latest_migration_in_directory(connection, Path::new(DB_MIGRATIONS_PATH)) {
				Ok(_) => (),
				Err(diesel::migrations::RunMigrationsError::MigrationError(diesel::migrations::MigrationError::NoMigrationRun)) => break,
				Err(e) => bail!(e),
			}
		}
		Ok(())
	}

	fn migrate_up(&self) -> Result<()> {
		let connection = self.connection.lock().unwrap();
		let connection = connection.deref();
		embedded_migrations::run(connection)?;
		Ok(())
	}

	fn virtualize_song(&self, mut song: Song) -> Option<Song> {
		song.path = match self.vfs.real_to_virtual(Path::new(&song.path)) {
			Ok(p) => p.to_string_lossy().into_owned(),
			_ => return None,
		};
		if let Some(artwork_path) = song.artwork {
			song.artwork = match self.vfs.real_to_virtual(Path::new(&artwork_path)) {
				Ok(p) => Some(p.to_string_lossy().into_owned()),
				_ => None,
			};
		}
		Some(song)
	}

	fn virtualize_directory(&self, mut directory: Directory) -> Option<Directory> {
		directory.path = match self.vfs.real_to_virtual(Path::new(&directory.path)) {
			Ok(p) => p.to_string_lossy().into_owned(),
			_ => return None,
		};
		if let Some(artwork_path) = directory.artwork {
			directory.artwork = match self.vfs.real_to_virtual(Path::new(&artwork_path)) {
				Ok(p) => Some(p.to_string_lossy().into_owned()),
				_ => None,
			};
		}
		Some(directory)
	}

	pub fn browse(&self, virtual_path: &Path) -> Result<Vec<CollectionFile>> {
		let mut output = Vec::new();
		let connection = self.connection.lock().unwrap();
		let connection = connection.deref();

		if virtual_path.components().count() == 0 {
			// Browse top-level
			let real_directories: Vec<Directory> = directories::table
				.filter(directories::columns::parent.is_null())
				.load(connection)?;
			let virtual_directories = real_directories
				.into_iter()
				.filter_map(|s| self.virtualize_directory(s));
			output.extend(virtual_directories
			                  .into_iter()
			                  .map(|d| CollectionFile::Directory(d)));

		} else {
			// Browse sub-directory
			let real_path = self.vfs.virtual_to_real(virtual_path)?;
			let real_path_string = real_path.as_path().to_string_lossy().into_owned();

			let real_songs: Vec<Song> = songs::table
				.filter(songs::columns::parent.eq(&real_path_string))
				.load(connection)?;
			let virtual_songs = real_songs
				.into_iter()
				.filter_map(|s| self.virtualize_song(s));
			output.extend(virtual_songs.map(|s| CollectionFile::Song(s)));

			let real_directories: Vec<Directory> = directories::table
				.filter(directories::columns::parent.eq(&real_path_string))
				.load(connection)?;
			let virtual_directories = real_directories
				.into_iter()
				.filter_map(|s| self.virtualize_directory(s));
			output.extend(virtual_directories.map(|d| CollectionFile::Directory(d)));
		}

		Ok(output)
	}

	pub fn flatten(&self, virtual_path: &Path) -> Result<Vec<Song>> {
		let connection = self.connection.lock().unwrap();
		let connection = connection.deref();
		let real_path = self.vfs.virtual_to_real(virtual_path)?;
		let like_path = real_path.as_path().to_string_lossy().into_owned() + "%";
		let real_songs: Vec<Song> = songs::table
			.filter(songs::columns::path.like(&like_path))
			.load(connection)?;
		let virtual_songs = real_songs
			.into_iter()
			.filter_map(|s| self.virtualize_song(s));
		Ok(virtual_songs.collect::<Vec<_>>())
	}

	pub fn get_random_albums(&self, count: i64) -> Result<Vec<Directory>> {
		let connection = self.connection.lock().unwrap();
		let connection = connection.deref();
		let real_directories = directories::table
			.filter(directories::columns::album.is_not_null())
			.limit(count)
			.order(sql::<types::Bool>("RANDOM()"))
			.load(connection)?;
		let virtual_directories = real_directories
			.into_iter()
			.filter_map(|s| self.virtualize_directory(s));
		Ok(virtual_directories.collect::<Vec<_>>())
	}

	pub fn get_recent_albums(&self, count: i64) -> Result<Vec<Directory>> {
		let connection = self.connection.lock().unwrap();
		let connection = connection.deref();
		let real_directories: Vec<Directory> = directories::table
			.filter(directories::columns::album.is_not_null())
			.order(directories::columns::date_added.desc())
			.limit(count)
			.load(connection)?;
		let virtual_directories = real_directories
			.into_iter()
			.filter_map(|s| self.virtualize_directory(s));
		Ok(virtual_directories.collect::<Vec<_>>())
	}
}

fn _get_test_db(name: &str) -> DB {
	use vfs::VfsConfig;
	use std::collections::HashMap;

	let mut collection_path = PathBuf::new();
	collection_path.push("test");
	collection_path.push("collection");
	let mut mount_points = HashMap::new();
	mount_points.insert("root".to_owned(), collection_path);
	let vfs = Arc::new(Vfs::new(VfsConfig { mount_points: mount_points }));

	let mut index_config = IndexConfig::new();
	index_config.path = PathBuf::new();
	index_config.path.push("test");
	index_config.path.push(name);

	if index_config.path.exists() {
		fs::remove_file(&index_config.path).unwrap();
	}

	DB::new(vfs, &index_config).unwrap()
}

#[test]
fn test_migrations_up() {
	_get_test_db("migrations_up.sqlite");
}

#[test]
fn test_migrations_down() {
	let db = _get_test_db("migrations_down.sqlite");
	db.migrate_down().unwrap();
	db.migrate_up().unwrap();
}

#[test]
fn test_browse_top_level() {
	let mut root_path = PathBuf::new();
	root_path.push("root");

	let db = _get_test_db("browse_top_level.sqlite");
	db.get_index().update_index().unwrap();
	let results = db.browse(Path::new("")).unwrap();

	assert_eq!(results.len(), 1);
	match results[0] {
		CollectionFile::Directory(ref d) => assert_eq!(d.path, root_path.to_str().unwrap()),
		_ => panic!("Expected directory"),
	}
}

#[test]
fn test_browse() {
	let mut khemmis_path = PathBuf::new();
	khemmis_path.push("root");
	khemmis_path.push("Khemmis");

	let mut tobokegao_path = PathBuf::new();
	tobokegao_path.push("root");
	tobokegao_path.push("Tobokegao");

	let db = _get_test_db("browse.sqlite");
	db.get_index().update_index().unwrap();
	let results = db.browse(Path::new("root")).unwrap();

	assert_eq!(results.len(), 2);
	match results[0] {
		CollectionFile::Directory(ref d) => assert_eq!(d.path, khemmis_path.to_str().unwrap()),
		_ => panic!("Expected directory"),
	}
	match results[1] {
		CollectionFile::Directory(ref d) => assert_eq!(d.path, tobokegao_path.to_str().unwrap()),
		_ => panic!("Expected directory"),
	}
}

#[test]
fn test_flatten() {
	let db = _get_test_db("flatten.sqlite");
	db.get_index().update_index().unwrap();
	let results = db.flatten(Path::new("root")).unwrap();
	assert_eq!(results.len(), 12);
}

#[test]
fn test_random() {
	let db = _get_test_db("random.sqlite");
	db.get_index().update_index().unwrap();
	let results = db.get_random_albums(1).unwrap();
	assert_eq!(results.len(), 1);
}

#[test]
fn test_recent() {
	let db = _get_test_db("recent.sqlite");
	db.get_index().update_index().unwrap();
	let results = db.get_recent_albums(2).unwrap();
	assert_eq!(results.len(), 2);
	assert!(results[0].date_added >= results[1].date_added);
}
