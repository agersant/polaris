use std::fs;
use std::path::Path;

use vfs::*;
use error::*;

#[derive(Debug, RustcEncodable)]
pub struct Song {
    path: String,
    display_name: String,
}

impl Song {
    pub fn read(collection: &Collection, file: &fs::DirEntry) -> Result<Song, CollectionError> {
        let file_meta = try!(file.metadata());
        assert!(file_meta.is_file());

        let file_path = file.path();
        let file_path = file_path.as_path();
        let virtual_path = try!(collection.vfs.real_to_virtual(file_path));
        let path_string = try!(virtual_path.to_str().ok_or(CollectionError::PathDecoding));

        let display_name = virtual_path.file_stem().unwrap();
        let display_name = display_name.to_str().unwrap();
        let display_name = display_name.to_string();

        Ok(Song {
            path: path_string.to_string(),
            display_name: display_name,
        })
    }
}

#[derive(Debug, RustcEncodable)]
pub struct Directory {
    path: String,
    display_name: String,
}

impl Directory {
    pub fn read(collection: &Collection,
                file: &fs::DirEntry)
                -> Result<Directory, CollectionError> {
        let file_meta = try!(file.metadata());
        assert!(file_meta.is_dir());

        let file_path = file.path();
        let file_path = file_path.as_path();
        let virtual_path = try!(collection.vfs.real_to_virtual(file_path));
        let path_string = try!(virtual_path.to_str().ok_or(CollectionError::PathDecoding));

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

impl Collection {
    pub fn new() -> Collection {
        Collection { vfs: Vfs::new() }
    }

    pub fn mount(&mut self, name: &str, real_path: &Path) -> Result<(), CollectionError> {
        self.vfs.mount(name, real_path)
    }

    pub fn browse(&self, path: &Path) -> Result<Vec<CollectionFile>, CollectionError> {

        let full_path = try!(self.vfs.virtual_to_real(path));

        let mut out = vec![];
        for file in try!(fs::read_dir(full_path)) {
            let file = try!(file);
            let file_meta = try!(file.metadata());
            if file_meta.is_file() {
                let song = try!(Song::read(self, &file));
                out.push(CollectionFile::Song(song));
            } else if file_meta.is_dir() {
                let directory = try!(Directory::read(self, &file));
                out.push(CollectionFile::Directory(directory));
            }
        }

        Ok(out)
    }

    fn flatten_internal(&self, path: &Path) -> Result<Vec<Song>, CollectionError> {
        let files = try!(fs::read_dir(path));
        files.fold(Ok(vec![]), |acc, file| {
            let mut acc = try!(acc);
            let file: fs::DirEntry = try!(file);
            let file_meta = try!(file.metadata());
            if file_meta.is_file() {
                let song = try!(Song::read(self, &file));
                acc.push(song);
            } else {
                let explore_path = file.path();
                let explore_path = explore_path.as_path();
                let mut explore_content = try!(self.flatten_internal(explore_path));
                acc.append(&mut explore_content);
            }
            Ok(acc)
        })
    }

    pub fn flatten(&self, path: &Path) -> Result<Vec<Song>, CollectionError> {
        let real_path = try!(self.vfs.virtual_to_real(path));
        self.flatten_internal(real_path.as_path())
    }
}
