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

const INDEX_BUILDING_INSERT_BUFFER_SIZE: usize = 500; // Put 500 insertions in each transaction
const INDEX_LOCK_TIMEOUT: usize = 100; // In milliseconds

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

#[derive(Debug, RustcEncodable)]
pub struct Song {
    path: String,
    track_number: Option<u32>,
    title: Option<String>,
    artist: Option<String>,
    album_artist: Option<String>,
    album: Option<String>,
    year: Option<i32>,
    artwork: Option<String>,
}

#[derive(Debug, RustcEncodable)]
pub struct Directory {
    path: String,
    artist: Option<String>,
    album: Option<String>,
    year: Option<i32>,
    artwork: Option<String>,
}

#[derive(Debug, RustcEncodable)]
pub enum CollectionFile {
    Directory(Directory),
    Song(Song),
}

struct IndexBuilder<'db> {
	queue: Vec<CollectionFile>,
	db: &'db sqlite::Connection,
	insert_directory: sqlite::Statement<'db>,
	insert_song: sqlite::Statement<'db>,
}

impl<'db> IndexBuilder<'db> {
	fn new(db: &sqlite::Connection) -> IndexBuilder {
		let mut queue = Vec::new();
		queue.reserve_exact(INDEX_BUILDING_INSERT_BUFFER_SIZE);
		IndexBuilder {
			queue: queue,
			db: db,
			insert_directory: db.prepare("INSERT OR REPLACE INTO directories (path, parent, artwork, year, artist, album) VALUES (?, ?, ?, ?, ?, ?)").unwrap(),
			insert_song: db.prepare("INSERT OR REPLACE INTO songs (path, parent, track_number, title, year, album_artist, artist, album, artwork) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)").unwrap(),
		}
	}

	fn get_parent(path: &str) -> Option<String> {
		let parent_path = Path::new(path);
		if let Some(parent_dir) = parent_path.parent() {
			if let Some(parent_path) = parent_dir.to_str() {
				return Some(parent_path.to_owned());
			}
		}
		None
	}

	fn flush(&mut self) {
		self.db.execute("BEGIN TRANSACTION").ok();
		while let Some(file) = self.queue.pop() {
			match file {

				// Insert directory
				CollectionFile::Directory(directory) => {
					let parent = IndexBuilder::get_parent(directory.path.as_str());
					self.insert_directory.reset().ok();
					self.insert_directory.bind(1, &sqlite::Value::String(directory.path)).unwrap();
					self.insert_directory.bind(2, &parent.map_or(sqlite::Value::Null, |t| sqlite::Value::String(t.to_owned()))).unwrap();
					self.insert_directory.bind(3, &directory.artwork.map_or(sqlite::Value::Null, |t| sqlite::Value::String(t.to_owned()))).unwrap();
					self.insert_directory.bind(4, &directory.year.map_or(sqlite::Value::Null, |t| sqlite::Value::Integer(t as i64))).unwrap();
					self.insert_directory.bind(5, &directory.artist.map_or(sqlite::Value::Null, |t| sqlite::Value::String(t))).unwrap();
					self.insert_directory.bind(6, &directory.album.map_or(sqlite::Value::Null, |t| sqlite::Value::String(t))).unwrap();
					self.insert_directory.next().ok();
				},

				// Insert song
				CollectionFile::Song(song) => {
					let parent = IndexBuilder::get_parent(song.path.as_str());
					self.insert_song.reset().ok();
					self.insert_song.bind(1, &sqlite::Value::String(song.path)).unwrap();
					self.insert_song.bind(2, &parent.map_or(sqlite::Value::Null, |t| sqlite::Value::String(t.to_owned()))).unwrap();
					self.insert_song.bind(3, &song.track_number.map_or(sqlite::Value::Null, |t| sqlite::Value::Integer(t as i64))).unwrap();
					self.insert_song.bind(4, &song.title.map_or(sqlite::Value::Null, |t| sqlite::Value::String(t))).unwrap();
					self.insert_song.bind(5, &song.year.map_or(sqlite::Value::Null, |t| sqlite::Value::Integer(t as i64))).unwrap();
					self.insert_song.bind(6, &song.album_artist.map_or(sqlite::Value::Null, |t| sqlite::Value::String(t))).unwrap();
					self.insert_song.bind(7, &song.artist.map_or(sqlite::Value::Null, |t| sqlite::Value::String(t))).unwrap();
					self.insert_song.bind(8, &song.album.map_or(sqlite::Value::Null, |t| sqlite::Value::String(t))).unwrap();
					self.insert_song.bind(9, &song.artwork.map_or(sqlite::Value::Null, |t| sqlite::Value::String(t.to_owned()))).unwrap();
					self.insert_song.next().ok();
				}

			}
		}
		self.db.execute("END TRANSACTION").ok();
	}

	fn push(&mut self, file: CollectionFile) {
		if self.queue.len() == self.queue.capacity() {
			self.flush();
		}
		self.queue.push(file);
	}
}

impl<'db> Drop for IndexBuilder<'db> {
	fn drop(&mut self) {
        self.flush();
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
		db.execute("

			CREATE TABLE version
			(	id INTEGER PRIMARY KEY NOT NULL
			,	number INTEGER NULL
			);
			INSERT INTO version (number) VALUES(1);

			CREATE TABLE directories
			(	id INTEGER PRIMARY KEY NOT NULL
			,	path TEXT NOT NULL
			,	parent TEXT
			,	artist TEXT
			,	year INTEGER
			,	album TEXT
			,	artwork TEXT
			,	UNIQUE(path)
			);

    		CREATE TABLE songs
			(	id INTEGER PRIMARY KEY NOT NULL
			,	path TEXT NOT NULL
			, 	parent TEXT NOT NULL
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
		let mut db = sqlite::open(self.path.clone()).unwrap();
		db.set_busy_timeout(INDEX_LOCK_TIMEOUT).ok();
		db
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
			let mut select = db.prepare("SELECT path FROM songs").unwrap();
			let mut delete = db.prepare("DELETE FROM songs WHERE path = ?").unwrap();
			while let sqlite::State::Row = select.next().unwrap() {
				let path_string : String = select.read(0).unwrap();
				let path = Path::new(path_string.as_str());
				if !path.exists() || self.vfs.real_to_virtual(path).is_err() {
					delete.reset().ok();
					delete.bind(1, &sqlite::Value::String(path_string.to_owned())).ok();
					delete.next().ok();
				}
			}
		}

		{
			let mut select = db.prepare("SELECT path FROM directories").unwrap();
			let mut delete = db.prepare("DELETE FROM directories WHERE path = ?").unwrap();
			while let sqlite::State::Row = select.next().unwrap() {
				let path_string : String = select.read(0).unwrap();
				let path = Path::new(path_string.as_str());
				if !path.exists() || self.vfs.real_to_virtual(path).is_err() {
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
		let mut builder = IndexBuilder::new(db);
		for (_, target) in mount_points {
			self.populate_directory(&mut builder, target.as_path());
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

	fn populate_directory(&self, builder: &mut IndexBuilder, path: &Path) {
		
		// Find artwork
		let artwork = self.get_artwork(path);

		let path_string = match path.to_str() {
			Some(p) => p,
			_ => return,
		};
		
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
					self.populate_directory(builder, file_path.as_path());
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

							let song = Song {
								path: file_path_string.to_owned(),
								track_number: tags.track_number,
								title: tags.title,
								artist: tags.artist,
								album_artist: tags.album_artist,
								album: tags.album,
								year: tags.year,
								artwork: artwork.as_ref().map(|s| s.to_owned()),
							};

							builder.push(CollectionFile::Song(song));
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

		let directory = Directory {
			path: path_string.to_owned(),
			artwork: artwork,
			album: directory_album,
			artist: directory_artist,
			year: directory_year,
		};
		builder.push(CollectionFile::Directory(directory));
	}

	pub fn run(&self)
	{
		loop {
			{
				let db = self.connect();
				self.update_index(&db);
			}
			thread::sleep(time::Duration::from_secs(60 * 20)); // TODO expose in configuration
		}
	}

	fn select_songs(&self, select: &mut sqlite::Statement) -> Vec<Song> {

		let mut output = Vec::new();

		while let sqlite::State::Row = select.next().unwrap() {

			let song_path : String = select.read(0).unwrap();
			let track_number : sqlite::Value = select.read(1).unwrap();
			let title : sqlite::Value = select.read(2).unwrap();
			let year : sqlite::Value = select.read(3).unwrap();
			let album_artist : sqlite::Value = select.read(4).unwrap();
			let artist : sqlite::Value = select.read(5).unwrap();
			let album : sqlite::Value = select.read(6).unwrap();
			let artwork : sqlite::Value = select.read(7).unwrap();

			let song_path = Path::new(song_path.as_str());
			let song_path = match self.vfs.real_to_virtual(song_path) {
				Ok(p) => p,
				_ => continue,
			};

			let artwork = artwork.as_string().map(|p| Path::new(p)).and_then(|p| self.vfs.real_to_virtual(p).ok());

			let song = Song {
				path: song_path.to_str().unwrap().to_owned(),
				track_number: track_number.as_integer().map(|n| n as u32),
				title: title.as_string().map(|s| s.to_owned()),
				year: year.as_integer().map(|n| n as i32),
				album_artist: album_artist.as_string().map(|s| s.to_owned()),
				artist: artist.as_string().map(|s| s.to_owned()),
				album: album.as_string().map(|s| s.to_owned()),
				artwork: artwork.map(|p| p.to_str().unwrap().to_owned() ),
			};
			output.push(song);
		}

		output
	}

	// List sub-directories within a directory
	fn browse_directories(&self, real_path: &Path) -> Vec<CollectionFile> {
		let db = self.connect();
		let mut output = Vec::new();

		let path_string = real_path.to_string_lossy();
		let mut select = db.prepare("SELECT path, artwork, year, artist, album FROM directories WHERE parent = ?").unwrap();
		select.bind(1, &sqlite::Value::String(path_string.deref().to_owned())).unwrap();

		while let sqlite::State::Row = select.next().unwrap() {

			let directory_value : String = select.read(0).unwrap();
			let artwork_path : sqlite::Value = select.read(1).unwrap();
			let year : sqlite::Value = select.read(2).unwrap();
			let artist : sqlite::Value = select.read(3).unwrap();
			let album : sqlite::Value = select.read(4).unwrap();

			let directory_path = Path::new(directory_value.as_str());
			let directory_path = match self.vfs.real_to_virtual(directory_path) {
				Ok(p) => p,
				_ => continue,
			};

			let artwork_path = artwork_path.as_string()
								.map(|p| Path::new(p))
								.and_then(|p| self.vfs.real_to_virtual(p).ok());

			let directory = Directory {
				path: directory_path.to_str().unwrap().to_owned(),
				artwork: artwork_path.map(|p| p.to_str().unwrap().to_owned() ),
				year: year.as_integer().map(|n| n as i32),
				artist: artist.as_string().map(|s| s.to_owned()),
				album: album.as_string().map(|s| s.to_owned()),
			};
			output.push(CollectionFile::Directory(directory));
		}

		output
	}

	// List songs within a directory
	fn browse_songs(&self, real_path: &Path) -> Vec<CollectionFile> {
		let db = self.connect();
		let path_string = real_path.to_string_lossy();
		let mut select = db.prepare("SELECT path, track_number, title, year, album_artist, artist, album, artwork FROM songs WHERE parent = ?").unwrap();
		select.bind(1, &sqlite::Value::String(path_string.deref().to_owned())).unwrap();
		self.select_songs(&mut select).into_iter().map(|s| CollectionFile::Song(s)).collect()
	}

	pub fn browse(&self, virtual_path: &Path) -> Result<Vec<CollectionFile>, PError> {

		let mut output = Vec::new();

		// Browse top-level
		if virtual_path.components().count() == 0 {
			for (n, _) in self.vfs.get_mount_points() {
				let directory = Directory {
					path: n.to_owned(),
					artwork: None,
					year: None,
					artist: None,
					album: None,
				};
				output.push(CollectionFile::Directory(directory));
			}

		// Browse sub-directory
		} else {
			let real_path = try!(self.vfs.virtual_to_real(virtual_path));
			let directories = self.browse_directories(real_path.as_path());
			let songs = self.browse_songs(real_path.as_path());
			output.extend(directories);
			output.extend(songs);
		}

		Ok(output)
	}

	pub fn flatten(&self, virtual_path: &Path) -> Result<Vec<Song>, PError> {
		let db = self.connect();
		let real_path = try!(self.vfs.virtual_to_real(virtual_path));
		let path_string = real_path.to_string_lossy().into_owned() + "%";
		let mut select = db.prepare("SELECT path, track_number, title, year, album_artist, artist, album, artwork FROM songs WHERE path LIKE ?").unwrap();
		select.bind(1, &sqlite::Value::String(path_string.deref().to_owned())).unwrap();
		Ok(self.select_songs(&mut select))
	}
}
