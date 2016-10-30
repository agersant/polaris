use std::fs;
use std::path::Path;
use std::path::PathBuf;
use regex::Regex;
use std::sync::Arc;

use config::Config;
use index::Index;
use vfs::*;
use error::*;
use utils::*;

#[derive(Debug, RustcEncodable)]
pub struct Album {
    title: Option<String>,
    year: Option<i32>,
    album_art: Option<String>,
    artist: Option<String>,
}

#[derive(Debug, RustcEncodable)]
pub struct Song {
    path: String,
    album: Album,
    track_number: Option<u32>,
    title: Option<String>,
    artist: Option<String>,
}

#[derive(Clone, Debug)]
pub struct User {
    name: String,
    password: String,
}

impl User {
    pub fn new(name: String, password: String) -> User {
        User {
            name: name,
            password: password,
        }
    }
}

impl Album {
    fn read(collection: &Collection, real_path: &Path) -> Result<Album, PError> {

        let album_art = collection.get_album_art(real_path).unwrap_or(None);
        let album_art = match album_art {
            Some(p) => Some(try!(collection.vfs.real_to_virtual(p.as_path()))),
            None => None,
        };
        let album_art = match album_art {
            None => None,
            Some(a) => a.to_str().map(|p| p.to_string()),
        };

        Ok(Album {
            album_art: album_art,
            title: None,
            year: None,
            artist: None,
        })
    }
}

impl Song {
    fn read(collection: &Collection, path: &Path) -> Result<Song, PError> {
        let virtual_path = try!(collection.vfs.real_to_virtual(path));
        let path_string = try!(virtual_path.to_str().ok_or(PError::PathDecoding));

        let album = try!(Album::read(collection, path));

        Ok(Song {
            path: path_string.to_string(),
            album: album,
            artist: None,
            title: None,
            track_number: None,
        })
    }

    
}

#[derive(Debug, RustcEncodable)]
pub struct Directory {
    path: String,
    name: String,
    album: Album,
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
    vfs: Arc<Vfs>,
    users: Vec<User>,
    album_art_pattern: Option<Regex>,
    index: Arc<Index>,
}

impl Collection {
    pub fn new(vfs: Arc<Vfs>, index: Arc<Index>) -> Collection {
        Collection {
            vfs: vfs,
            users: Vec::new(),
            album_art_pattern: None,
            index: index,
        }
    }

    pub fn load_config(&mut self, config: &Config) -> Result<(), PError> {
        self.album_art_pattern = config.album_art_pattern.clone();
        self.users = config.users.to_vec();
        Ok(())
    }

    pub fn auth(&self, username: &str, password: &str) -> bool {
        self.users.iter().any(|u| u.name == username && u.password == password)
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
                    if is_song(file_path) {
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
                if is_song(file_path) {
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
        let pattern = match self.album_art_pattern {
            Some(ref p) => p,
            None => return Ok(None),
        };

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
            pattern.is_match(file_name)
        });

        match album_art {
            Some(Err(_)) => Err(PError::AlbumArtSearchError),
            Some(Ok(a)) => Ok(Some(a.path())),
            None => Ok(None),
        }
    }
}
