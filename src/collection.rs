use std::io::prelude::*;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;
use toml;

use vfs::*;
use error::*;

#[derive(Debug, RustcEncodable)]
pub struct Song {
    path: String,
    display_name: String,
}

impl Song {
    fn read(collection: &Collection, path: &Path) -> Result<Song, PError> {
        let virtual_path = try!(collection.vfs.real_to_virtual(path));
        let path_string = try!(virtual_path.to_str().ok_or(PError::PathDecoding));

        let display_name = virtual_path.file_stem().unwrap();
        let display_name = display_name.to_str().unwrap();
        let display_name = display_name.to_string();

        Ok(Song {
            path: path_string.to_string(),
            display_name: display_name,
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
    display_name: String,
}

impl Directory {
    fn read(collection: &Collection, path: &Path) -> Result<Directory, PError> {
        let virtual_path = try!(collection.vfs.real_to_virtual(path));
        let path_string = try!(virtual_path.to_str().ok_or(PError::PathDecoding));

        let display_name = virtual_path.iter().last().unwrap();
        let display_name = display_name.to_str().unwrap();
        let display_name = display_name.to_string();

        Ok(Directory {
            path: path_string.to_string(),
            display_name: display_name,
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
}

const CONFIG_MOUNT_DIRS: &'static str = "mount_dirs";
const CONFIG_MOUNT_DIR_NAME: &'static str = "name";
const CONFIG_MOUNT_DIR_SOURCE: &'static str = "source";

impl Collection {
    pub fn new() -> Collection {
        Collection { vfs: Vfs::new() }
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
}
