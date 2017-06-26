use core::ops::Deref;
use diesel;
use diesel::expression::sql;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel::types;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;

use db::models::*;
use db::schema::{directories, songs};
use errors::*;
use metadata;
use vfs::Vfs;

#[allow(dead_code)]
const DB_MIGRATIONS_PATH: &'static str = "src/db/migrations";
embed_migrations!("src/db/migrations");

const INDEX_BUILDING_INSERT_BUFFER_SIZE: usize = 1000; // Insertions in each transaction

pub struct IndexConfig {
	pub album_art_pattern: Option<Regex>,
	pub sleep_duration: u64, // in seconds
	pub path: PathBuf,
}

impl IndexConfig {
	pub fn new() -> IndexConfig {
		IndexConfig {
			sleep_duration: 60 * 30, // 30 minutes
			album_art_pattern: None,
			path: Path::new(":memory:").to_path_buf(),
		}
	}
}

pub struct Index {
	vfs: Arc<Vfs>,
	db: Mutex<SqliteConnection>,
	album_art_pattern: Option<Regex>,
	sleep_duration: u64,
}

#[derive(Debug, Insertable)]
#[table_name="songs"]
struct NewSong {
	path: String,
	parent: String,
	track_number: Option<i32>,
	disc_number: Option<i32>,
	title: Option<String>,
	artist: Option<String>,
	album_artist: Option<String>,
	year: Option<i32>,
	album: Option<String>,
	artwork: Option<String>,
}

#[derive(Debug, Insertable)]
#[table_name="directories"]
struct NewDirectory {
	path: String,
	parent: Option<String>,
	artist: Option<String>,
	year: Option<i32>,
	album: Option<String>,
	artwork: Option<String>,
	date_added: i32,
}

struct IndexBuilder<'db> {
	new_songs: Vec<NewSong>,
	new_directories: Vec<NewDirectory>,
	db: &'db Mutex<SqliteConnection>,
}

impl<'db> IndexBuilder<'db> {
	fn new(db: &Mutex<SqliteConnection>) -> Result<IndexBuilder> {
		let mut new_songs = Vec::new();
		let mut new_directories = Vec::new();
		new_songs.reserve_exact(INDEX_BUILDING_INSERT_BUFFER_SIZE);
		new_directories.reserve_exact(INDEX_BUILDING_INSERT_BUFFER_SIZE);
		Ok(IndexBuilder {
		       new_songs: new_songs,
		       new_directories: new_directories,
		       db: db,
		   })
	}

	fn flush_songs(&mut self) -> Result<()> {
		let db = self.db.lock().unwrap();
		let db = db.deref();
		db.transaction::<_, Error, _>(|| {
				                            diesel::insert(&self.new_songs)
				                                .into(songs::table)
				                                .execute(db)?;
				                            Ok(())
				                           })?;
		self.new_songs.clear();
		Ok(())
	}

	fn flush_directories(&mut self) -> Result<()> {
		let db = self.db.lock().unwrap();
		let db = db.deref();
		db.transaction::<_, Error, _>(|| {
				                            diesel::insert(&self.new_directories)
				                                .into(directories::table)
				                                .execute(db)?;
				                            Ok(())
				                           })?;
		self.new_directories.clear();
		Ok(())
	}

	fn push_song(&mut self, song: NewSong) -> Result<()> {
		if self.new_songs.len() >= self.new_songs.capacity() {
			self.flush_songs()?;
		}
		self.new_songs.push(song);
		Ok(())
	}

	fn push_directory(&mut self, directory: NewDirectory) -> Result<()> {
		if self.new_directories.len() >= self.new_directories.capacity() {
			self.flush_directories()?;
		}
		self.new_directories.push(directory);
		Ok(())
	}
}

impl Index {
	pub fn new(vfs: Arc<Vfs>, config: &IndexConfig) -> Result<Index> {

		let path = &config.path;

		println!("Index file path: {}", path.to_string_lossy());

		let db = Mutex::new(SqliteConnection::establish(&path.to_string_lossy())?);

		let index = Index {
			vfs: vfs,
			db: db,
			album_art_pattern: config.album_art_pattern.clone(),
			sleep_duration: config.sleep_duration,
		};

		index.init()?;

		Ok(index)
	}

	fn init(&self) -> Result<()> {
		{
			let db = self.db.lock().unwrap();
			db.execute("PRAGMA synchronous = NORMAL")?;
		}
		self.migrate_up()?;
		Ok(())
	}

	#[allow(dead_code)]
	fn migrate_down(&self) -> Result<()> {
		let db = self.db.lock().unwrap();
		let db = db.deref();
		loop {
			match diesel::migrations::revert_latest_migration_in_directory(db, Path::new(DB_MIGRATIONS_PATH)) {
				Ok(_) => (),
				Err(diesel::migrations::RunMigrationsError::MigrationError(diesel::migrations::MigrationError::NoMigrationRun)) => break,
				Err(e) => bail!(e),
			}
		}
		Ok(())
	}

	fn migrate_up(&self) -> Result<()> {
		let db = self.db.lock().unwrap();
		let db = db.deref();
		embedded_migrations::run(db)?;
		Ok(())
	}

	fn update_index(&self) -> Result<()> {
		let start = time::Instant::now();
		println!("Beginning library index update");
		self.clean()?;
		self.populate()?;
		println!("Library index update took {} seconds",
		         start.elapsed().as_secs());
		Ok(())
	}

	fn clean(&self) -> Result<()> {
		{
			let all_songs: Vec<String>;
			{
				let db = self.db.lock().unwrap();
				let db = db.deref();
				all_songs = songs::table.select(songs::columns::path).load(db)?;
			}

			let missing_songs = all_songs
				.into_iter()
				.filter(|ref song_path| {
					        let path = Path::new(&song_path);
					        !path.exists() || self.vfs.real_to_virtual(path).is_err()
					       })
				.collect::<Vec<_>>();

			let db = self.db.lock().unwrap();
			let db = db.deref();
			diesel::delete(songs::table.filter(songs::columns::path.eq_any(missing_songs)))
				.execute(db)?;
		}

		{
			let all_directories: Vec<String>;
			{
				let db = self.db.lock().unwrap();
				let db = db.deref();
				all_directories = directories::table
					.select(directories::columns::path)
					.load(db)?;
			}

			let missing_directories = all_directories
				.into_iter()
				.filter(|ref directory_path| {
					        let path = Path::new(&directory_path);
					        !path.exists() || self.vfs.real_to_virtual(path).is_err()
					       })
				.collect::<Vec<_>>();

			let db = self.db.lock().unwrap();
			let db = db.deref();
			diesel::delete(directories::table.filter(directories::columns::path
			                                             .eq_any(missing_directories)))
					.execute(db)?;
		}

		Ok(())
	}

	fn populate(&self) -> Result<()> {
		let vfs = self.vfs.deref();
		let mount_points = vfs.get_mount_points();
		let mut builder = IndexBuilder::new(&self.db)?;
		for (_, target) in mount_points {
			self.populate_directory(&mut builder, None, target.as_path())?;
		}
		builder.flush_songs()?;
		builder.flush_directories()?;
		Ok(())
	}

	fn get_artwork(&self, dir: &Path) -> Option<String> {
		let pattern = match self.album_art_pattern {
			Some(ref p) => p,
			_ => return None,
		};

		if let Ok(dir_content) = fs::read_dir(dir) {
			for file in dir_content {
				if let Ok(file) = file {
					if let Some(name_string) = file.file_name().to_str() {
						if pattern.is_match(name_string) {
							return file.path().to_str().map(|p| p.to_owned());
						}
					}
				}
			}
		}

		None
	}

	fn populate_directory(&self,
	                      builder: &mut IndexBuilder,
	                      parent: Option<&Path>,
	                      path: &Path)
	                      -> Result<()> {

		// Find artwork
		let artwork = self.get_artwork(path);

		// Extract path and parent path
		let parent_string = parent.and_then(|p| p.to_str()).map(|s| s.to_owned());
		let path_string = path.to_str().ok_or("Invalid directory path")?;

		// Find date added
		let metadata = fs::metadata(path_string)?;
		let created = metadata
			.created()
			.or(metadata.modified())?
			.duration_since(time::UNIX_EPOCH)?
			.as_secs() as i32;

		let mut directory_album = None;
		let mut directory_year = None;
		let mut directory_artist = None;
		let mut inconsistent_directory_album = false;
		let mut inconsistent_directory_year = false;
		let mut inconsistent_directory_artist = false;

		// Insert content
		if let Ok(dir_content) = fs::read_dir(path) {
			for file in dir_content {
				let file_path = match file {
					Ok(f) => f.path(),
					_ => continue,
				};

				if file_path.is_dir() {
					self.populate_directory(builder, Some(path), file_path.as_path())?;
				} else {
					if let Some(file_path_string) = file_path.to_str() {
						if let Ok(tags) = metadata::read(file_path.as_path()) {
							if tags.year.is_some() {
								inconsistent_directory_year |= directory_year.is_some() &&
								                               directory_year != tags.year;
								directory_year = tags.year;
							}

							if tags.album.is_some() {
								inconsistent_directory_album |= directory_album.is_some() &&
								                                directory_album != tags.album;
								directory_album = tags.album.as_ref().map(|a| a.clone());
							}

							if tags.album_artist.is_some() {
								inconsistent_directory_artist |= directory_artist.is_some() &&
								                                 directory_artist !=
								                                 tags.album_artist;
								directory_artist = tags.album_artist.as_ref().map(|a| a.clone());
							} else if tags.artist.is_some() {
								inconsistent_directory_artist |= directory_artist.is_some() &&
								                                 directory_artist != tags.artist;
								directory_artist = tags.artist.as_ref().map(|a| a.clone());
							}

							let song = NewSong {
								path: file_path_string.to_owned(),
								parent: path_string.to_owned(),
								disc_number: tags.disc_number.map(|n| n as i32),
								track_number: tags.track_number.map(|n| n as i32),
								title: tags.title,
								artist: tags.artist,
								album_artist: tags.album_artist,
								album: tags.album,
								year: tags.year,
								artwork: artwork.as_ref().map(|s| s.to_owned()),
							};

							builder.push_song(song)?;
						}
					}
				}
			}
		}

		// Insert directory
		if inconsistent_directory_year {
			directory_year = None;
		}
		if inconsistent_directory_album {
			directory_album = None;
		}
		if inconsistent_directory_artist {
			directory_artist = None;
		}

		let directory = NewDirectory {
			path: path_string.to_owned(),
			parent: parent_string,
			artwork: artwork,
			album: directory_album,
			artist: directory_artist,
			year: directory_year,
			date_added: created,
		};
		builder.push_directory(directory)?;

		Ok(())
	}

	pub fn run(&self) {
		loop {
			if let Err(e) = self.update_index() {
				println!("Error while updating index: {}", e);
			}
			thread::sleep(time::Duration::from_secs(self.sleep_duration));
		}
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
		let db = self.db.lock().unwrap();
		let db = db.deref();

		// Browse top-level
		if virtual_path.components().count() == 0 {
			let real_directories: Vec<Directory> = directories::table
				.filter(directories::columns::parent.is_null())
				.load(db)?;
			let virtual_directories = real_directories
				.into_iter()
				.filter_map(|s| self.virtualize_directory(s));
			output.extend(virtual_directories
			                  .into_iter()
			                  .map(|d| CollectionFile::Directory(d)));

			// Browse sub-directory
		} else {
			let real_path = self.vfs.virtual_to_real(virtual_path)?;
			let real_path_string = real_path.as_path().to_string_lossy().into_owned();

			let real_songs: Vec<Song> = songs::table
				.filter(songs::columns::parent.eq(&real_path_string))
				.load(db)?;
			let virtual_songs = real_songs
				.into_iter()
				.filter_map(|s| self.virtualize_song(s));
			output.extend(virtual_songs.map(|s| CollectionFile::Song(s)));

			let real_directories: Vec<Directory> = directories::table
				.filter(directories::columns::parent.eq(&real_path_string))
				.load(db)?;
			let virtual_directories = real_directories
				.into_iter()
				.filter_map(|s| self.virtualize_directory(s));
			output.extend(virtual_directories.map(|d| CollectionFile::Directory(d)));
		}

		Ok(output)
	}

	pub fn flatten(&self, virtual_path: &Path) -> Result<Vec<Song>> {
		let db = self.db.lock().unwrap();
		let db = db.deref();
		let real_path = self.vfs.virtual_to_real(virtual_path)?;
		let like_path = real_path.as_path().to_string_lossy().into_owned() + "%";
		let real_songs: Vec<Song> = songs::table
			.filter(songs::columns::path.like(&like_path))
			.load(db)?;
		let virtual_songs = real_songs
			.into_iter()
			.filter_map(|s| self.virtualize_song(s));
		Ok(virtual_songs.collect::<Vec<_>>())
	}

	pub fn get_random_albums(&self, count: i64) -> Result<Vec<Directory>> {
		let db = self.db.lock().unwrap();
		let db = db.deref();
		let real_directories = directories::table
			.filter(directories::columns::album.is_not_null())
			.limit(count)
			.order(sql::<types::Bool>("RANDOM()"))
			.load(db)?;
		let virtual_directories = real_directories
			.into_iter()
			.filter_map(|s| self.virtualize_directory(s));
		Ok(virtual_directories.collect::<Vec<_>>())
	}

	pub fn get_recent_albums(&self, count: i64) -> Result<Vec<Directory>> {
		let db = self.db.lock().unwrap();
		let db = db.deref();
		let real_directories: Vec<Directory> = directories::table
			.filter(directories::columns::album.is_not_null())
			.order(directories::columns::date_added.desc())
			.limit(count)
			.load(db)?;
		let virtual_directories = real_directories
			.into_iter()
			.filter_map(|s| self.virtualize_directory(s));
		Ok(virtual_directories.collect::<Vec<_>>())
	}
}

fn _get_test_index(name: &str) -> Index {
	use vfs::VfsConfig;
	use std::collections::HashMap;

	let mut collection_path = PathBuf::new();
	collection_path.push("test");
	collection_path.push("collection");
	let mut mount_points = HashMap::new();
	mount_points.insert("root".to_owned(), collection_path);

	let vfs = Arc::new(Vfs::new(VfsConfig { mount_points: mount_points }));

	let mut index_config = IndexConfig::new();
	index_config.album_art_pattern = Some(Regex::new(r#"^Folder\.(png|jpg|jpeg)$"#).unwrap());
	index_config.path = PathBuf::new();
	index_config.path.push("test");
	index_config.path.push(name);

	if index_config.path.exists() {
		fs::remove_file(&index_config.path).unwrap();
	}

	Index::new(vfs, &index_config).unwrap()
}

#[test]
fn test_migrations_up() {
	_get_test_index("migrations_up.sqlite");
}

#[test]
fn test_migrations_down() {
	let index = _get_test_index("migrations_down.sqlite");
	index.migrate_down().unwrap();
	index.migrate_up().unwrap();
}

#[test]
fn test_populate() {
	let index = _get_test_index("populate.sqlite");
	index.update_index().unwrap();
	index.update_index().unwrap(); // Check that subsequent updates don't run into conflicts

	let db = index.db.lock().unwrap();
	let db = db.deref();
	let all_directories: Vec<Directory> = directories::table.load(db).unwrap();
	let all_songs: Vec<Song> = songs::table.load(db).unwrap();
	assert_eq!(all_directories.len(), 5);
	assert_eq!(all_songs.len(), 12);
}

#[test]
fn test_metadata() {
	let mut target = PathBuf::new();
	target.push("root");
	target.push("Tobokegao");
	target.push("Picnic");

	let mut song_path = target.clone();
	song_path.push("05 - シャーベット (Sherbet).mp3");

	let mut artwork_path = target.clone();
	artwork_path.push("Folder.png");

	let index = _get_test_index("metadata.sqlite");
	index.update_index().unwrap();
	let results = index.flatten(target.as_path()).unwrap();

	assert_eq!(results.len(), 7);
	let song = &results[4];
	assert_eq!(song.path, song_path.to_string_lossy().as_ref());
	assert_eq!(song.track_number, Some(5));
	assert_eq!(song.disc_number, None);
	assert_eq!(song.title, Some("シャーベット (Sherbet)".to_owned()));
	assert_eq!(song.artist, Some("Tobokegao".to_owned()));
	assert_eq!(song.album_artist, None);
	assert_eq!(song.album, Some("Picnic".to_owned()));
	assert_eq!(song.year, Some(2016));
	assert_eq!(song.artwork,
	           Some(artwork_path.to_string_lossy().into_owned()));
}

#[test]
fn test_browse_top_level() {
	let mut root_path = PathBuf::new();
	root_path.push("root");

	let index = _get_test_index("browse_top_level.sqlite");
	index.update_index().unwrap();
	let results = index.browse(Path::new("")).unwrap();

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

	let index = _get_test_index("browse.sqlite");
	index.update_index().unwrap();
	let results = index.browse(Path::new("root")).unwrap();

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
	let index = _get_test_index("flatten.sqlite");
	index.update_index().unwrap();
	let results = index.flatten(Path::new("root")).unwrap();
	assert_eq!(results.len(), 12);
}

#[test]
fn test_random() {
	let index = _get_test_index("random.sqlite");
	index.update_index().unwrap();
	let results = index.get_random_albums(1).unwrap();
	assert_eq!(results.len(), 1);
}

#[test]
fn test_recent() {
	let index = _get_test_index("recent.sqlite");
	index.update_index().unwrap();
	let results = index.get_recent_albums(2).unwrap();
	assert_eq!(results.len(), 2);
	assert!(results[0].date_added >= results[1].date_added);
}
