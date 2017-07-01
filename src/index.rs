use core::ops::Deref;
use diesel;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::thread;
use std::time;

use config::{MiscSettings, UserConfig};
use db::ConnectionSource;
use db::DB;
use db::{directories, misc_settings, songs};
use vfs::VFSSource;
use errors::*;
use metadata;

const INDEX_BUILDING_INSERT_BUFFER_SIZE: usize = 1000; // Insertions in each transaction
const INDEX_BUILDING_CLEAN_BUFFER_SIZE: usize = 500; // Insertions in each transaction

#[derive(Debug, Queryable, Serialize)]
pub struct Song {
	#[serde(skip_serializing)]
	id: i32,
	pub path: String,
	#[serde(skip_serializing)]
	pub parent: String,
	pub track_number: Option<i32>,
	pub disc_number: Option<i32>,
	pub title: Option<String>,
	pub artist: Option<String>,
	pub album_artist: Option<String>,
	pub year: Option<i32>,
	pub album: Option<String>,
	pub artwork: Option<String>,
}

#[derive(Debug, Queryable, Serialize)]
pub struct Directory {
	#[serde(skip_serializing)]
	id: i32,
	pub path: String,
	#[serde(skip_serializing)]
	pub parent: Option<String>,
	pub artist: Option<String>,
	pub year: Option<i32>,
	pub album: Option<String>,
	pub artwork: Option<String>,
	pub date_added: i32,
}

#[derive(Debug, Serialize)]
pub enum CollectionFile {
	Directory(Directory),
	Song(Song),
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

struct IndexBuilder<'conn> {
	new_songs: Vec<NewSong>,
	new_directories: Vec<NewDirectory>,
	connection: &'conn Mutex<SqliteConnection>,
	album_art_pattern: Regex,
}

impl<'conn> IndexBuilder<'conn> {
	fn new(connection: &Mutex<SqliteConnection>, album_art_pattern: Regex) -> Result<IndexBuilder> {
		let mut new_songs = Vec::new();
		let mut new_directories = Vec::new();
		new_songs.reserve_exact(INDEX_BUILDING_INSERT_BUFFER_SIZE);
		new_directories.reserve_exact(INDEX_BUILDING_INSERT_BUFFER_SIZE);
		Ok(IndexBuilder {
		       new_songs: new_songs,
		       new_directories: new_directories,
		       connection: connection,
		       album_art_pattern: album_art_pattern,
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

	fn get_artwork(&self, dir: &Path) -> Option<String> {
		if let Ok(dir_content) = fs::read_dir(dir) {
			for file in dir_content {
				if let Ok(file) = file {
					if let Some(name_string) = file.file_name().to_str() {
						if self.album_art_pattern.is_match(name_string) {
							return file.path().to_str().map(|p| p.to_owned());
						}
					}
				}
			}
		}

		None
	}

	fn populate_directory(&mut self, parent: Option<&Path>, path: &Path) -> Result<()> {

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

		// Sub directories
		let mut sub_directories = Vec::new();

		// Insert content
		if let Ok(dir_content) = fs::read_dir(path) {
			for file in dir_content {
				let file_path = match file {
					Ok(f) => f.path(),
					_ => continue,
				};

				if file_path.is_dir() {
					sub_directories.push(file_path.to_path_buf());
					continue;
				}

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
							                                 directory_artist != tags.album_artist;
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

						self.push_song(song)?;
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
		self.push_directory(directory)?;

		// Populate subdirectories
		for sub_directory in sub_directories {
			self.populate_directory(Some(path), &sub_directory)?;
		}

		Ok(())
	}
}

fn clean<T>(db: &T) -> Result<()> where T: ConnectionSource + VFSSource {
	let vfs = db.get_vfs()?;

	{
		let all_songs: Vec<String>;
		{
			let connection = db.get_connection();
			let connection = connection.lock().unwrap();
			let connection = connection.deref();
			all_songs = songs::table.select(songs::path).load(connection)?;
		}

		let missing_songs = all_songs
			.into_iter()
			.filter(|ref song_path| {
				        let path = Path::new(&song_path);
				        !path.exists() || vfs.real_to_virtual(path).is_err()
				       })
			.collect::<Vec<_>>();

		{
			let connection = db.get_connection();
			let connection = connection.lock().unwrap();
			let connection = connection.deref();
			for chunk in missing_songs[..].chunks(INDEX_BUILDING_CLEAN_BUFFER_SIZE) {
				diesel::delete(songs::table.filter(songs::path.eq_any(chunk)))
					.execute(connection)?;
			}

		}
	}

	{
		let all_directories: Vec<String>;
		{
			let connection = db.get_connection();
			let connection = connection.lock().unwrap();
			let connection = connection.deref();
			all_directories = directories::table
				.select(directories::path)
				.load(connection)?;
		}

		let missing_directories = all_directories
			.into_iter()
			.filter(|ref directory_path| {
				        let path = Path::new(&directory_path);
				        !path.exists() || vfs.real_to_virtual(path).is_err()
				       })
			.collect::<Vec<_>>();

		{
			let connection = db.get_connection();
			let connection = connection.lock().unwrap();
			let connection = connection.deref();
			for chunk in missing_directories[..].chunks(INDEX_BUILDING_CLEAN_BUFFER_SIZE) {
				diesel::delete(directories::table.filter(directories::path.eq_any(chunk)))
					.execute(connection)?;
			}
		}
	}

	Ok(())
}

fn populate<T>(db: &T) -> Result<()> where T: ConnectionSource + VFSSource {
	let vfs = db.get_vfs()?;
	let mount_points = vfs.get_mount_points();
	let connection = db.get_connection();

	let album_art_pattern;
	{
		let connection = connection.lock().unwrap();
		let connection = connection.deref();
		let settings: MiscSettings = misc_settings::table.get_result(connection)?;
		album_art_pattern = Regex::new(&settings.index_album_art_pattern)?;
	}

	let mut builder = IndexBuilder::new(&connection, album_art_pattern)?;
	for (_, target) in mount_points {
		builder.populate_directory(None, target.as_path())?;
	}
	builder.flush_songs()?;
	builder.flush_directories()?;
	Ok(())
}

pub fn update<T>(db: &T) -> Result<()> where T: ConnectionSource + VFSSource {
	let start = time::Instant::now();
	println!("Beginning library index update");
	clean(db)?;
	populate(db)?;
	println!("Library index update took {} seconds",
	         start.elapsed().as_secs());
	Ok(())
}

pub fn update_loop<T>(db: &T) where T: ConnectionSource + VFSSource {
	loop {
		if let Err(e) = update(db) {
			println!("Error while updating index: {}", e);
		}
		{
			let sleep_duration;
			{
				let connection = db.get_connection();
				let connection = connection.lock().unwrap();
				let connection = connection.deref();
				let settings: Result<MiscSettings> = misc_settings::table
					.get_result(connection)
					.map_err(|e| e.into());
				if let Err(ref e) = settings {
					println!("Could not retrieve index sleep duration: {}", e);
				}
				sleep_duration = settings
					.map(|s| s.index_sleep_duration_seconds)
					.unwrap_or(1800);
			}
			thread::sleep(time::Duration::from_secs(sleep_duration as u64));
		}
	}
}

fn _get_test_db(name: &str) -> DB  {
	let config_path = Path::new("test/config.toml");
	let config = UserConfig::parse(&config_path).unwrap();

	let mut db_path = PathBuf::new();
	db_path.push("test");
	db_path.push(name);
	if db_path.exists() {
		fs::remove_file(&db_path).unwrap();
	}

	let db = DB::new(&db_path).unwrap();
	db.load_config(&config).unwrap();
	db
}

#[test]
fn test_populate() {
	let db = _get_test_db("populate.sqlite");
	update(&db).unwrap();
	update(&db).unwrap(); // Check that subsequent updates don't run into conflicts

	let connection = db.get_connection();
	let connection = connection.lock().unwrap();
	let connection = connection.deref();
	let all_directories: Vec<Directory> = directories::table.load(connection).unwrap();
	let all_songs: Vec<Song> = songs::table.load(connection).unwrap();
	assert_eq!(all_directories.len(), 5);
	assert_eq!(all_songs.len(), 12);
}

#[test]
fn test_metadata() {
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
	update(&db).unwrap();

	let connection = db.get_connection();
	let connection = connection.lock().unwrap();
	let connection = connection.deref();
	let songs: Vec<Song> = songs::table
		.filter(songs::title.eq("シャーベット (Sherbet)"))
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
