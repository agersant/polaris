use std::io::prelude::*;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;
use regex::Regex;
use toml;

use vfs::*;
use error::*;

#[derive(Debug, RustcEncodable)]
pub struct Album {
    name: Option<String>,
    year: Option<String>,
    album_art: Option<String>,
    artist: Option<String>,
}

impl Album {
    fn read(collection: &Collection, path: &Path) -> Result<Option<Album>, PError> {
        let name = None;
        let year = None;
        let artist = None;

        let album_art = collection.get_album_art(path).unwrap_or(None);
        let album_art = match album_art {
            Some(p) => Some(try!(collection.vfs.real_to_virtual(p.as_path()))),
            None => None,
        };
        let album_art = match album_art {
            None => None,
            Some(a) => a.to_str().map(|p| p.to_string()),
        };

        Ok(Some(Album {
            name: name,
            year: year,
            album_art: album_art,
            artist: artist,
        }))
    }
}

#[derive(Debug, RustcEncodable)]
pub struct Song {
    path: String,
    album: Album,
    title: Option<String>,
    artist: Option<String>,
}

impl Song {
    fn read(collection: &Collection, path: &Path) -> Result<Song, PError> {
        let virtual_path = try!(collection.vfs.real_to_virtual(path));
        let path_string = try!(virtual_path.to_str().ok_or(PError::PathDecoding));

        let name = virtual_path.file_stem().unwrap();
        let name = name.to_str().unwrap();
        let name = name.to_string();

        let album = try!(Album::read(collection, path));
        let album = album.unwrap();

        Ok(Song {
            path: path_string.to_string(),
            album: album,
            artist: None,
            title: Some(name),
        })
    }

    fn is_song(path: &Path) -> bool {
        let extension = match path.extension() {
            Some(e) => e,
            _ => return false,
        };
        let extension = match extension.to_str() {
            Some(e) => e,
            _ => return false,
        };
        match extension {
            "mp3" => return true,
            "ogg" => return true,
            "m4a" => return true,
            "flac" => return true,
            _ => return false,
        }
    }
}

#[derive(Debug, RustcEncodable)]
pub struct Directory {
    path: String,
    name: String,
    album: Option<Album>,
}

impl Directory {
    fn read(collection: &Collection, path: &Path) -> Result<Directory, PError> {
        let virtual_path = try!(collection.vfs.real_to_virtual(path));
        let path_string = try!(virtual_path.to_str().ok_or(PError::PathDecoding));

        let name = virtual_path.iter().last().unwrap();
        let name = name.to_str().unwrap();
        let name = name.to_string();

        let album = try!(Album::read(collection, path));

        Ok(Directory {
            path: path_string.to_string(),
            name: name,
            album: album,
        })
    }
}

#[derive(Debug, RustcEncodable)]
pub enum CollectionFile {
    Directory(Directory),
    Song(Song),
}

pub struct Collection {
    vfs: Vfs,
    album_art_pattern: Regex,
}

const CONFIG_MOUNT_DIRS: &'static str = "mount_dirs";
const CONFIG_MOUNT_DIR_NAME: &'static str = "name";
const CONFIG_MOUNT_DIR_SOURCE: &'static str = "source";
const CONFIG_ALBUM_ART_PATTERN: &'static str = "album_art_pattern";

impl Collection {
    pub fn new() -> Collection {
        Collection {
            vfs: Vfs::new(),
            album_art_pattern: Regex::new("^Folder\\.png$").unwrap(),
        }
    }

    pub fn load_config(&mut self, config_path: &Path) -> Result<(), PError> {
        // Open
        let mut config_file = match File::open(config_path) {
            Ok(c) => c,
            Err(_) => return Err(PError::ConfigFileOpenError),
        };

        // Read
        let mut config_file_content = String::new();
        match config_file.read_to_string(&mut config_file_content) {
            Ok(_) => (),
            Err(_) => return Err(PError::ConfigFileReadError),
        };

        // Parse
        let parsed_config = toml::Parser::new(config_file_content.as_str()).parse();
        let parsed_config = match parsed_config {
            Some(c) => c,
            None => return Err(PError::ConfigFileParseError),
        };

        // Apply
        try!(self.load_config_mount_points(&parsed_config));
        try!(self.load_config_album_art_pattern(&parsed_config));

        Ok(())
    }

    fn load_config_album_art_pattern(&mut self, config: &toml::Table) -> Result<(), PError> {
        let pattern = match config.get(CONFIG_ALBUM_ART_PATTERN) {
            Some(s) => s,
            None => return Ok(()),
        };
        let pattern = match pattern {
            &toml::Value::String(ref s) => s,
            _ => return Err(PError::ConfigAlbumArtPatternParseError),
        };
        self.album_art_pattern = match Regex::new(pattern) {
            Ok(r) => r,
            Err(_) => return Err(PError::ConfigAlbumArtPatternParseError),
        };

        Ok(())
    }

    fn load_config_mount_points(&mut self, config: &toml::Table) -> Result<(), PError> {
        let mount_dirs = match config.get(CONFIG_MOUNT_DIRS) {
            Some(s) => s,
            None => return Ok(()),
        };

        let mount_dirs = match mount_dirs {
            &toml::Value::Array(ref a) => a,
            _ => return Err(PError::ConfigMountDirsParseError),
        };

        for dir in mount_dirs {
            let name = match dir.lookup(CONFIG_MOUNT_DIR_NAME) {
                None => return Err(PError::ConfigMountDirsParseError),
                Some(n) => n,
            };
            let name = match name.as_str() {
                None => return Err(PError::ConfigMountDirsParseError),
                Some(n) => n,
            };

            let source = match dir.lookup(CONFIG_MOUNT_DIR_SOURCE) {
                None => return Err(PError::ConfigMountDirsParseError),
                Some(n) => n,
            };
            let source = match source.as_str() {
                None => return Err(PError::ConfigMountDirsParseError),
                Some(n) => n,
            };
            let source = PathBuf::from(source);

            try!(self.mount(name, source.as_path()));
        }

        Ok(())
    }

    fn mount(&mut self, name: &str, real_path: &Path) -> Result<(), PError> {
        self.vfs.mount(name, real_path)
    }

    pub fn browse(&self, path: &Path) -> Result<Vec<CollectionFile>, PError> {

        let mut out = vec![];

        if path.components().count() == 0 {
            let mount_points = self.vfs.get_mount_points();
            for (_, target) in mount_points {
                let directory = try!(Directory::read(self, target.as_path()));
                out.push(CollectionFile::Directory(directory));
            }
        } else {
            let full_path = try!(self.vfs.virtual_to_real(path));
            for file in try!(fs::read_dir(full_path)) {
                let file = try!(file);
                let file_meta = try!(file.metadata());
                let file_path = file.path();
                let file_path = file_path.as_path();
                if file_meta.is_file() {
                    if Song::is_song( file_path ) {
                        let song = try!(Song::read(self, file_path));
                        out.push(CollectionFile::Song(song));
                    }
                } else if file_meta.is_dir() {
                    let directory = try!(Directory::read(self, file_path));
                    out.push(CollectionFile::Directory(directory));
                }
            }
        }

        Ok(out)
    }

    fn flatten_internal(&self, path: &Path) -> Result<Vec<Song>, PError> {
        let files = try!(fs::read_dir(path));
        files.fold(Ok(vec![]), |acc, file| {
            let mut acc = try!(acc);
            let file: fs::DirEntry = try!(file);
            let file_meta = try!(file.metadata());
            let file_path = file.path();
            let file_path = file_path.as_path();
            if file_meta.is_file() {
                if Song::is_song(file_path) {
                    let song = try!(Song::read(self, file_path));
                    acc.push(song);
                }
            } else {
                let mut explore_content = try!(self.flatten_internal(file_path));
                acc.append(&mut explore_content);
            }
            Ok(acc)
        })
    }

    pub fn flatten(&self, path: &Path) -> Result<Vec<Song>, PError> {
        let real_path = try!(self.vfs.virtual_to_real(path));
        self.flatten_internal(real_path.as_path())
    }

    pub fn locate(&self, virtual_path: &Path) -> Result<PathBuf, PError> {
        self.vfs.virtual_to_real(virtual_path)
    }

    fn get_album_art(&self, real_path: &Path) -> Result<Option<PathBuf>, PError> {
        let mut real_dir = real_path;
        if real_dir.is_file() {
            real_dir = try!(real_dir.parent().ok_or(PError::AlbumArtSearchError));
        }
        assert!(real_dir.is_dir());

        let mut files = try!(fs::read_dir(real_dir));
        let album_art = files.find(|dir_entry| {
            let file = match *dir_entry {
                Err(_) => return false,
                Ok(ref r) => r,
            };
            let file_name = file.file_name();
            let file_name = match file_name.to_str() {
                None => return false,
                Some(r) => r,
            };
            self.album_art_pattern.is_match(file_name)
        });

        match album_art {
            Some(Err(_)) => Err(PError::AlbumArtSearchError),
            Some(Ok(a)) => Ok(Some(a.path())),
            None => Ok(None),
        }
    }
}
