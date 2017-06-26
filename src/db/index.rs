use core::ops::Deref;
use diesel;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;

use db::DB;
use db::schema::{directories, songs};
use errors::*;
use metadata;
use vfs::Vfs;

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
	connection: &'db Mutex<SqliteConnection>,
}

impl<'db> IndexBuilder<'db> {
	fn new(connection: &Mutex<SqliteConnection>) -> Result<IndexBuilder> {
		let mut new_songs = Vec::new();
		let mut new_directories = Vec::new();
		new_songs.reserve_exact(INDEX_BUILDING_INSERT_BUFFER_SIZE);
		new_directories.reserve_exact(INDEX_BUILDING_INSERT_BUFFER_SIZE);
		Ok(IndexBuilder {
		       new_songs: new_songs,
		       new_directories: new_directories,
		       connection: connection,
		   })
	}

	fn flush_songs(&mut self) -> Result<()> {
		let connection = self.connection.lock().unwrap();
		let connection = connection.deref();
		connection
			.transaction::<_, Error, _>(|| {
				                            diesel::insert(&self.new_songs)
				                                .into(songs::table)
				                                .execute(connection)?;
				                            Ok(())
				                           })?;
		self.new_songs.clear();
		Ok(())
	}

	fn flush_directories(&mut self) -> Result<()> {
		let connection = self.connection.lock().unwrap();
		let connection = connection.deref();
		connection
			.transaction::<_, Error, _>(|| {
				                            diesel::insert(&self.new_directories)
				                                .into(directories::table)
				                                .execute(connection)?;
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

pub struct Index {
	vfs: Arc<Vfs>,
	connection: Arc<Mutex<SqliteConnection>>,
	album_art_pattern: Option<Regex>,
	sleep_duration: u64,
}

impl Index {
	pub fn new(vfs: Arc<Vfs>,
	           connection: Arc<Mutex<SqliteConnection>>,
	           config: &IndexConfig)
	           -> Index {
		let index = Index {
			vfs: vfs,
			connection: connection,
			album_art_pattern: config.album_art_pattern.clone(),
			sleep_duration: config.sleep_duration,
		};
		index
	}

	pub fn update_index(&self) -> Result<()> {
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
				let connection = self.connection.lock().unwrap();
				let connection = connection.deref();
				all_songs = songs::table
					.select(songs::columns::path)
					.load(connection)?;
			}

			let missing_songs = all_songs
				.into_iter()
				.filter(|ref song_path| {
					        let path = Path::new(&song_path);
					        !path.exists() || self.vfs.real_to_virtual(path).is_err()
					       })
				.collect::<Vec<_>>();

			let connection = self.connection.lock().unwrap();
			let connection = connection.deref();
			diesel::delete(songs::table.filter(songs::columns::path.eq_any(missing_songs)))
				.execute(connection)?;
		}

		{
			let all_directories: Vec<String>;
			{
				let connection = self.connection.lock().unwrap();
				let connection = connection.deref();
				all_directories = directories::table
					.select(directories::columns::path)
					.load(connection)?;
			}

			let missing_directories = all_directories
				.into_iter()
				.filter(|ref directory_path| {
					        let path = Path::new(&directory_path);
					        !path.exists() || self.vfs.real_to_virtual(path).is_err()
					       })
				.collect::<Vec<_>>();

			let connection = self.connection.lock().unwrap();
			let connection = connection.deref();
			diesel::delete(directories::table.filter(directories::columns::path
			                                             .eq_any(missing_directories)))
					.execute(connection)?;
		}

		Ok(())
	}

	fn populate(&self) -> Result<()> {
		let vfs = self.vfs.deref();
		let mount_points = vfs.get_mount_points();
		let mut builder = IndexBuilder::new(&self.connection)?;
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

	pub fn update_loop(&self) {
		loop {
			if let Err(e) = self.update_index() {
				println!("Error while updating index: {}", e);
			}
			thread::sleep(time::Duration::from_secs(self.sleep_duration));
		}
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
	index_config.album_art_pattern = Some(Regex::new(r#"^Folder\.(png|jpg|jpeg)$"#).unwrap());
	index_config.path = PathBuf::new();
	index_config.path.push("test");
	index_config.path.push(name);

	if index_config.path.exists() {
		fs::remove_file(&index_config.path).unwrap();
	}

	DB::new(vfs, &index_config).unwrap()
}

#[test]
fn test_populate() {
	use db::models::*;

	let db = _get_test_db("populate.sqlite");
	let index = db.get_index();
	index.update_index().unwrap();
	index.update_index().unwrap(); // Check that subsequent updates don't run into conflicts

	let connection = index.connection.lock().unwrap();
	let connection = connection.deref();
	let all_directories: Vec<Directory> = directories::table.load(connection).unwrap();
	let all_songs: Vec<Song> = songs::table.load(connection).unwrap();
	assert_eq!(all_directories.len(), 5);
	assert_eq!(all_songs.len(), 12);
}

#[test]
fn test_metadata() {
	use db::models::*;

	let mut target = PathBuf::new();
	target.push("test");
	target.push("collection");
	target.push("Tobokegao");
	target.push("Picnic");

	let mut song_path = target.clone();
	song_path.push("05 - シャーベット (Sherbet).mp3");

	let mut artwork_path = target.clone();
	artwork_path.push("Folder.png");

	let db = _get_test_db("metadata.sqlite");
	let index = db.get_index();
	index.update_index().unwrap();

	let connection = index.connection.lock().unwrap();
	let connection = connection.deref();
	let songs: Vec<Song> = songs::table
		.filter(songs::columns::title.eq("シャーベット (Sherbet)"))
		.load(connection)
		.unwrap();

	assert_eq!(songs.len(), 1);
	let song = &songs[0];
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
