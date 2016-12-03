use core::ops::Deref;
use regex::Regex;
use sqlite;
use sqlite::{Connection, State, Statement, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time;

use errors::*;
use metadata;
use vfs::Vfs;

const INDEX_BUILDING_INSERT_BUFFER_SIZE: usize = 250; // Insertions in each transaction
const INDEX_LOCK_TIMEOUT: usize = 1000; // In milliseconds

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
    path: String,
    vfs: Arc<Vfs>,
    album_art_pattern: Option<Regex>,
    sleep_duration: u64,
}

#[derive(Debug, RustcEncodable)]
pub struct Song {
    path: String,
    track_number: Option<u32>,
    disc_number: Option<u32>,
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

fn string_option_to_value(input: Option<String>) -> Value {
    match input {
        Some(s) => Value::String(s),
        None => Value::Null,
    }
}

fn i32_option_to_value(input: Option<i32>) -> Value {
    match input {
        Some(s) => Value::Integer(s as i64),
        None => Value::Null,
    }
}

fn u32_option_to_value(input: Option<u32>) -> Value {
    match input {
        Some(s) => Value::Integer(s as i64),
        None => Value::Null,
    }
}

struct IndexBuilder<'db> {
    queue: Vec<CollectionFile>,
    db: &'db Connection,
    insert_directory: Statement<'db>,
    insert_song: Statement<'db>,
}

impl<'db> IndexBuilder<'db> {
    fn new(db: &Connection) -> Result<IndexBuilder> {
        let mut queue = Vec::new();
        queue.reserve_exact(INDEX_BUILDING_INSERT_BUFFER_SIZE);
        Ok(IndexBuilder {
            queue: queue,
            db: db,
            insert_directory:
                db.prepare("INSERT OR REPLACE INTO directories (path, parent, artwork, year, \
                          artist, album) VALUES (?, ?, ?, ?, ?, ?)")?,
            insert_song:
                db.prepare("INSERT OR REPLACE INTO songs (path, parent, disc_number, track_number, \
                          title, year, album_artist, artist, album, artwork) VALUES (?, ?, ?, ?, \
                          ?, ?, ?, ?, ?, ?)")?,
        })
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

    fn flush(&mut self) -> Result<()> {
        self.db.execute("BEGIN TRANSACTION")?;
        while let Some(file) = self.queue.pop() {
            match file {

                // Insert directory
                CollectionFile::Directory(directory) => {
                    let parent = IndexBuilder::get_parent(directory.path.as_str());
                    self.insert_directory.reset()?;
                    self.insert_directory.bind(1, &Value::String(directory.path))?;
                    self.insert_directory.bind(2, &string_option_to_value(parent))?;
                    self.insert_directory
                        .bind(3, &string_option_to_value(directory.artwork))?;
                    self.insert_directory.bind(4, &i32_option_to_value(directory.year))?;
                    self.insert_directory
                        .bind(5, &string_option_to_value(directory.artist))?;
                    self.insert_directory
                        .bind(6, &string_option_to_value(directory.album))?;
                    self.insert_directory.next()?;
                }

                // Insert song
                CollectionFile::Song(song) => {
                    let parent = IndexBuilder::get_parent(song.path.as_str());
                    self.insert_song.reset()?;
                    self.insert_song.bind(1, &Value::String(song.path))?;
                    self.insert_song.bind(2, &string_option_to_value(parent))?;
                    self.insert_song.bind(3, &u32_option_to_value(song.disc_number))?;
                    self.insert_song.bind(4, &u32_option_to_value(song.track_number))?;
                    self.insert_song.bind(5, &string_option_to_value(song.title))?;
                    self.insert_song.bind(6, &i32_option_to_value(song.year))?;
                    self.insert_song.bind(7, &string_option_to_value(song.album_artist))?;
                    self.insert_song.bind(8, &string_option_to_value(song.artist))?;
                    self.insert_song.bind(9, &string_option_to_value(song.album))?;
                    self.insert_song.bind(10, &string_option_to_value(song.artwork))?;
                    self.insert_song.next()?;
                }

            }
        }
        self.db.execute("END TRANSACTION")?;
        Ok(())
    }

    fn push(&mut self, file: CollectionFile) -> Result<()> {
        if self.queue.len() == self.queue.capacity() {
            self.flush()?;
        }
        self.queue.push(file);
        Ok(())
    }
}

impl Index {
    pub fn new(vfs: Arc<Vfs>, config: &IndexConfig) -> Result<Index> {

        let path = &config.path;

        println!("Index file path: {}", path.to_string_lossy());

        let index = Index {
            path: path.to_string_lossy().deref().to_string(),
            vfs: vfs,
            album_art_pattern: config.album_art_pattern.clone(),
            sleep_duration: config.sleep_duration,
        };

        if path.exists() {
            // Migration
        } else {
            index.init()?;
        }

        Ok(index)
    }

    fn init(&self) -> Result<()> {

        println!("Initializing index database");

        let db = self.connect()?;
        db.execute("PRAGMA synchronous = NORMAL")?;
        db.execute("

			CREATE TABLE version
			(	id INTEGER PRIMARY KEY NOT NULL
			,	number \
                      INTEGER NULL
			);
			INSERT INTO version (number) VALUES(1);

			CREATE \
                      TABLE directories
			(	id INTEGER PRIMARY KEY NOT NULL
			,	path TEXT NOT \
                      NULL
			,	parent TEXT
			,	artist TEXT
			,	year INTEGER
			,	album TEXT
			\
                      ,	artwork TEXT
			,	UNIQUE(path)
			);

    		CREATE TABLE songs
			(	id \
                      INTEGER PRIMARY KEY NOT NULL
			,	path TEXT NOT NULL
			, 	parent TEXT NOT \
                      NULL
			,	disc_number INTEGER
			,	track_number INTEGER
			,	title TEXT
			\
                      ,	artist TEXT
			,	album_artist TEXT
			,	year INTEGER
			,	album TEXT
			\
                      ,	artwork TEXT
			,	UNIQUE(path)
			);

		")?;

        Ok(())
    }

    fn connect(&self) -> Result<Connection> {
        let mut db = sqlite::open(self.path.clone())?;
        db.set_busy_timeout(INDEX_LOCK_TIMEOUT)?;
        Ok(db)
    }

    fn update_index(&self, db: &Connection) -> Result<()> {
        let start = time::Instant::now();
        println!("Beginning library index update");
        self.clean(db)?;
        self.populate(db)?;
        println!("Library index update took {} seconds",
                 start.elapsed().as_secs());
        Ok(())
    }

    fn clean(&self, db: &Connection) -> Result<()> {
        {
            let mut select = db.prepare("SELECT path FROM songs")?;
            let mut delete = db.prepare("DELETE FROM songs WHERE path = ?")?;
            while let Ok(State::Row) = select.next() {
                let path_string: String = select.read(0)?;
                let path = Path::new(path_string.as_str());
                if !path.exists() || self.vfs.real_to_virtual(path).is_err() {
                    delete.reset()?;
                    delete.bind(1, &Value::String(path_string.to_owned()))?;
                    delete.next()?;
                }
            }
        }

        {
            let mut select = db.prepare("SELECT path FROM directories")?;
            let mut delete = db.prepare("DELETE FROM directories WHERE path = ?")?;
            while let Ok(State::Row) = select.next() {
                let path_string: String = select.read(0)?;
                let path = Path::new(path_string.as_str());
                if !path.exists() || self.vfs.real_to_virtual(path).is_err() {
                    delete.reset()?;
                    delete.bind(1, &Value::String(path_string.to_owned()))?;
                    delete.next()?;
                }
            }
        }

        Ok(())
    }

    fn populate(&self, db: &Connection) -> Result<()> {
        let vfs = self.vfs.deref();
        let mount_points = vfs.get_mount_points();
        let mut builder = IndexBuilder::new(db)?;
        for (_, target) in mount_points {
            self.populate_directory(&mut builder, target.as_path())?;
        }
        builder.flush()?;
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

    fn populate_directory(&self, builder: &mut IndexBuilder, path: &Path) -> Result<()> {

        // Find artwork
        let artwork = self.get_artwork(path);

        let path_string = path.to_str().ok_or("Invalid directory path")?;

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
                    self.populate_directory(builder, file_path.as_path())?;
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

                            let song = Song {
                                path: file_path_string.to_owned(),
                                disc_number: tags.disc_number,
                                track_number: tags.track_number,
                                title: tags.title,
                                artist: tags.artist,
                                album_artist: tags.album_artist,
                                album: tags.album,
                                year: tags.year,
                                artwork: artwork.as_ref().map(|s| s.to_owned()),
                            };

                            builder.push(CollectionFile::Song(song))?;
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
        builder.push(CollectionFile::Directory(directory))?;

        Ok(())
    }

    pub fn run(&self) {
        loop {
            if let Ok(db) = self.connect() {
                if let Err(e) = self.update_index(&db) {
                    println!("Error while updating index: {}", e);
                }
            }
            thread::sleep(time::Duration::from_secs(self.sleep_duration));
        }
    }

    fn select_songs(&self, select: &mut Statement) -> Result<Vec<Song>> {

        let mut output = Vec::new();

        while let State::Row = select.next()? {

            let song_path: String = select.read(0)?;
            let disc_number: Value = select.read(1)?;
            let track_number: Value = select.read(2)?;
            let title: Value = select.read(3)?;
            let year: Value = select.read(4)?;
            let album_artist: Value = select.read(5)?;
            let artist: Value = select.read(6)?;
            let album: Value = select.read(7)?;
            let artwork: Value = select.read(8)?;

            let song_path = Path::new(song_path.as_str());
            let song_path = match self.vfs.real_to_virtual(song_path) {
                Ok(p) => p.to_str().ok_or("Invalid song path")?.to_owned(),
                _ => continue,
            };

            let artwork_path = artwork.as_string().map(|p| Path::new(p));
            let mut artwork = None;
            if let Some(artwork_path) = artwork_path {
                artwork = match self.vfs.real_to_virtual(artwork_path) {
                    Ok(p) => Some(p.to_str().ok_or("Invalid song artwork path")?.to_owned()),
                    _ => None,
                };
            }

            let song = Song {
                path: song_path,
                disc_number: disc_number.as_integer().map(|n| n as u32),
                track_number: track_number.as_integer().map(|n| n as u32),
                title: title.as_string().map(|s| s.to_owned()),
                year: year.as_integer().map(|n| n as i32),
                album_artist: album_artist.as_string().map(|s| s.to_owned()),
                artist: artist.as_string().map(|s| s.to_owned()),
                album: album.as_string().map(|s| s.to_owned()),
                artwork: artwork,
            };
            output.push(song);
        }

        Ok(output)
    }

    // List sub-directories within a directory
    fn browse_directories(&self, real_path: &Path) -> Result<Vec<CollectionFile>> {
        let db = self.connect()?;
        let mut output = Vec::new();

        let path_string = real_path.to_string_lossy();
        let mut select =
            db.prepare("SELECT path, artwork, year, artist, album FROM directories WHERE \
                          parent = ? ORDER BY path COLLATE NOCASE ASC")?;
        select.bind(1, &Value::String(path_string.deref().to_owned()))?;

        while let State::Row = select.next()? {

            let directory_value: String = select.read(0)?;
            let artwork_path: Value = select.read(1)?;
            let year: Value = select.read(2)?;
            let artist: Value = select.read(3)?;
            let album: Value = select.read(4)?;

            let directory_path = Path::new(directory_value.as_str());
            let directory_path = match self.vfs.real_to_virtual(directory_path) {
                Ok(p) => p.to_str().ok_or("Invalid directory path")?.to_owned(),
                _ => continue,
            };

            let artwork_path = artwork_path.as_string().map(|p| Path::new(p));
            let mut artwork = None;
            if let Some(artwork_path) = artwork_path {
                artwork = match self.vfs.real_to_virtual(artwork_path) {
                    Ok(p) => Some(p.to_str().ok_or("Invalid directory artwork path")?.to_owned()),
                    _ => None,
                };
            }

            let directory = Directory {
                path: directory_path,
                artwork: artwork,
                year: year.as_integer().map(|n| n as i32),
                artist: artist.as_string().map(|s| s.to_owned()),
                album: album.as_string().map(|s| s.to_owned()),
            };
            output.push(CollectionFile::Directory(directory));
        }

        Ok(output)
    }

    // List songs within a directory
    fn browse_songs(&self, real_path: &Path) -> Result<Vec<CollectionFile>> {
        let db = self.connect()?;
        let path_string = real_path.to_string_lossy();
        let mut select =
            db.prepare("SELECT path, disc_number, track_number, title, year, album_artist, \
                          artist, album, artwork FROM songs WHERE parent = ? ORDER BY path \
                          COLLATE NOCASE ASC")?;
        select.bind(1, &Value::String(path_string.deref().to_owned()))?;
        Ok(self.select_songs(&mut select)?.into_iter().map(|s| CollectionFile::Song(s)).collect())
    }

    pub fn browse(&self, virtual_path: &Path) -> Result<Vec<CollectionFile>> {

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
            let real_path = self.vfs.virtual_to_real(virtual_path)?;
            let directories = self.browse_directories(real_path.as_path())?;
            let songs = self.browse_songs(real_path.as_path())?;
            output.extend(directories);
            output.extend(songs);
        }

        Ok(output)
    }

    pub fn flatten(&self, virtual_path: &Path) -> Result<Vec<Song>> {
        let db = self.connect()?;
        let real_path = self.vfs.virtual_to_real(virtual_path)?;
        let path_string = real_path.to_string_lossy().into_owned() + "%";
        let mut select =
            db.prepare("SELECT path, disc_number, track_number, title, year, album_artist, \
                          artist, album, artwork FROM songs WHERE path LIKE ? ORDER BY path \
                          COLLATE NOCASE ASC")?;
        select.bind(1, &Value::String(path_string.deref().to_owned()))?;
        self.select_songs(&mut select)
    }
}
