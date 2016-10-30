use sqlite;
use core::ops::Deref;
use id3::Tag;
use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time;

use error::*;
use vfs::Vfs;

pub struct Index {
	path: String,
	vfs: Arc<Vfs>,
	album_art_pattern: Option<Regex>,
}

struct SongTags {
    track_number: Option<u32>,
    title: Option<String>,
    artist: Option<String>,
    album_artist: Option<String>,
    album: Option<String>,
    year: Option<i32>,
}

impl SongTags {
    fn read(path: &Path) -> Result<SongTags, PError> {
        let tag = try!(Tag::read_from_path(path));

        let artist = tag.artist().map(|s| s.to_string());
        let album_artist = tag.album_artist().map(|s| s.to_string());
        let album = tag.album().map(|s| s.to_string());
        let title = tag.title().map(|s| s.to_string());
        let track_number = tag.track();
        let year = tag.year()
            .map(|y| y as i32)
            .or(tag.date_released().and_then(|d| d.year))
            .or(tag.date_recorded().and_then(|d| d.year));
		
        Ok(SongTags {
            artist: artist,
            album_artist: album_artist,
            album: album,
            title: title,
            track_number: track_number,
            year: year,
        })
    }
}

impl Index {

	pub fn new(path: &Path, vfs: Arc<Vfs>, album_art_pattern: &Option<Regex>) -> Result<Index, PError> {

		let index = Index {
			path: path.to_string_lossy().deref().to_string(),
			vfs: vfs,
			album_art_pattern: album_art_pattern.clone(),
		};

		if path.exists() {
			// Migration
		} else {
			index.init();
		}

		Ok(index)
	}

	fn init(&self) {

		println!("Initializing index database");

		let db = self.connect();
		db.execute("PRAGMA synchronous = NORMAL").unwrap();
		db.execute("PRAGMA journal_mode = WAL").unwrap();
		db.execute("

			CREATE TABLE version
			(	id INTEGER PRIMARY KEY NOT NULL
			,	number INTEGER NULL
			);
			INSERT INTO version (number) VALUES(1);

			CREATE TABLE directories
			(	id INTEGER PRIMARY KEY NOT NULL
			,	path NOT NULL
			,	artist TEXT
			,	year INTEGER
			,	album TEXT
			,	artwork TEXT
			,	UNIQUE(path)
			);

    		CREATE TABLE songs
			(	id INTEGER PRIMARY KEY NOT NULL
			,	path NOT NULL
			,	track_number INTEGER
			,	title TEXT
			,	artist TEXT
			,	album_artist TEXT
			,	year INTEGER
			,	album TEXT
			,	artwork TEXT
			,	UNIQUE(path)
			);

		").unwrap();
	}

	fn connect(&self) -> sqlite::Connection {
		sqlite::open(self.path.clone()).unwrap()
	}

	fn update_index(&self, db: &sqlite::Connection) {
		let start = time::Instant::now();
		println!("Indexing library");
		self.clean(db);
		self.populate(db);
		println!("Indexing library took {} seconds", start.elapsed().as_secs());	
	}

	fn clean(&self, db: &sqlite::Connection) {
		{
			let mut cursor = db.prepare("SELECT path FROM songs").unwrap().cursor();
			let mut delete = db.prepare("DELETE FROM songs WHERE path = ?").unwrap();
			while let Some(row) = cursor.next().unwrap() {
				let path_string = row[0].as_string().unwrap();
				let path = Path::new(path_string);
				if !path.exists() {
					delete.reset().ok();
					delete.bind(1, &sqlite::Value::String(path_string.to_owned())).ok();
					delete.next().ok();
				}
			}
		}

		{
			let mut cursor = db.prepare("SELECT path FROM directories").unwrap().cursor();
			let mut delete = db.prepare("DELETE FROM directories WHERE path = ?").unwrap();
			while let Some(row) = cursor.next().unwrap() {
				let path_string = row[0].as_string().unwrap();
				let path = Path::new(path_string);
				if !path.exists() {
					delete.reset().ok();
					delete.bind(1, &sqlite::Value::String(path_string.to_owned())).ok();
					delete.next().ok();
				}
			}
		}
	}

	fn populate(&self, db: &sqlite::Connection) {
		let vfs = self.vfs.deref();
		let mount_points = vfs.get_mount_points();

		for (_, target) in mount_points {
			self.populate_directory(&db, target.as_path());
		}
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

	fn populate_directory(&self, db: &sqlite::Connection, path: &Path) {
		
		// Find artwork
		let artwork = self.get_artwork(path).map_or(sqlite::Value::Null, |t| sqlite::Value::String(t));
		
		let mut directory_album = None;
		let mut directory_year = None;
		let mut directory_artist = None;
		let mut inconsistent_directory_album = false;
		let mut inconsistent_directory_year = false;
		let mut inconsistent_directory_artist = false;

		// Prepare statements
		let mut insert_directory = db.prepare("
			INSERT OR REPLACE INTO directories (path, artwork, year, artist, album)
			VALUES (?, ?, ?, ?, ?)
		").unwrap();

		let mut insert_song = db.prepare("
			INSERT OR REPLACE INTO songs (path, track_number, title, year, album_artist, artist, album, artwork)
			VALUES (?, ?, ?, ?, ?, ?, ?, ?)
		").unwrap();

		// Insert content
		if let Ok(dir_content) = fs::read_dir(path) {
			for file in dir_content {
				let file_path = match file {
					Ok(f) => f.path(),
					_ => continue,
				};

				if file_path.is_dir() {
					self.populate_directory(db, file_path.as_path());
				} else {
					if let Some(file_path_string) = file_path.to_str() {
						if let Ok(tags) = SongTags::read(file_path.as_path()) {
							if tags.year.is_some() {
								inconsistent_directory_year |= directory_year.is_some() && directory_year != tags.year;
								directory_year = tags.year;
							}

							if tags.album.is_some() {
								inconsistent_directory_album |= directory_album.is_some() && directory_album != tags.album;
								directory_album = Some(tags.album.as_ref().unwrap().clone());
							}

							if tags.album_artist.is_some() {
								inconsistent_directory_artist |= directory_artist.is_some() && directory_artist != tags.album_artist;
								directory_artist = Some(tags.album_artist.as_ref().unwrap().clone());
							} else if tags.artist.is_some() {
								inconsistent_directory_artist |= directory_artist.is_some() && directory_artist != tags.artist;
								directory_artist = Some(tags.artist.as_ref().unwrap().clone());
							}

							insert_song.reset().ok();
							insert_song.bind(1, &sqlite::Value::String(file_path_string.to_owned())).unwrap();
							insert_song.bind(2, &tags.track_number.map_or(sqlite::Value::Null, |t| sqlite::Value::Integer(t as i64))).unwrap();
							insert_song.bind(3, &tags.title.map_or(sqlite::Value::Null, |t| sqlite::Value::String(t))).unwrap();
							insert_song.bind(4, &tags.year.map_or(sqlite::Value::Null, |t| sqlite::Value::Integer(t as i64))).unwrap();
							insert_song.bind(5, &tags.album_artist.map_or(sqlite::Value::Null, |t| sqlite::Value::String(t))).unwrap();
							insert_song.bind(6, &tags.artist.map_or(sqlite::Value::Null, |t| sqlite::Value::String(t))).unwrap();
							insert_song.bind(7, &tags.album.map_or(sqlite::Value::Null, |t| sqlite::Value::String(t))).unwrap();
							insert_song.bind(8, &artwork).unwrap();
							insert_song.next().ok();
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
		if let Some(path_string) = path.to_str() {
			insert_directory.reset().ok();
			insert_directory.bind(1, &sqlite::Value::String(path_string.to_owned())).unwrap();
			insert_directory.bind(2, &artwork).unwrap();
			insert_directory.bind(3, &directory_year.map_or(sqlite::Value::Null, |t| sqlite::Value::Integer(t as i64))).unwrap();
			insert_directory.bind(4, &directory_artist.map_or(sqlite::Value::Null, |t| sqlite::Value::String(t))).unwrap();
			insert_directory.bind(5, &directory_album.map_or(sqlite::Value::Null, |t| sqlite::Value::String(t))).unwrap();
			insert_directory.next().ok();
		}
	}

	pub fn run(&self)
	{
		loop {
			let db = self.connect();
			if let Err(e) = db.execute("BEGIN TRANSACTION") {
				print!("Error while beginning transaction for index update: {}", e);
			} else {
				self.update_index(&db);
				if let Err(e) = db.execute("END TRANSACTION") {
					print!("Error while ending transaction for index update: {}", e);
				}
			}
			thread::sleep(time::Duration::from_secs(60 * 20)); // TODO expose in configuration
		}
	}
}
