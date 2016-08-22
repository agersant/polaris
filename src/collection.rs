use std::fs;
use std::path::Path;

use vfs::*;
use error::*;

#[derive(Debug, RustcEncodable)]
pub struct Song {
    path: String,
}

#[derive(Debug, RustcEncodable)]
pub struct Directory {
    path: String,
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
        Collection{
            vfs: Vfs::new(),
        }
    }

    pub fn mount(&mut self, name: &str, real_path: &Path) -> Result<(), CollectionError> {
        self.vfs.mount(name, real_path)
    }

    pub fn browse(&self, path: &Path) -> Result<Vec<CollectionFile>, CollectionError> {

        let full_path = try!(self.vfs.virtual_to_real(path));
        let full_path = full_path.to_str().unwrap();
        println!("Browsing: {}", full_path);

        let mut out = vec![];
        for file in try!(fs::read_dir(full_path)) {
            let file = try!(file);
            let file_meta = try!(file.metadata());
            let file_path = file.path();
            let file_path = file_path.as_path();
            if file_meta.is_file() {
                let virtual_path = try!(self.vfs.real_to_virtual(file_path));
                let path_string = try!(virtual_path.to_str().ok_or(CollectionError::PathDecoding)); 
                let collection_file = CollectionFile::Song(Song {
                    path: path_string.to_string(),
                });
                out.push(collection_file);
            } else if file_meta.is_dir() {
                let virtual_path = try!(self.vfs.real_to_virtual(file_path));
                let path_string = try!(virtual_path.to_str().ok_or(CollectionError::PathDecoding));  
                let collection_file = CollectionFile::Directory(Directory {
                    path: path_string.to_string(),
                });
                out.push(collection_file);
            }
        }

        Ok(out)
    }

    pub fn flatten(&self, path: &Path) -> Vec<CollectionFile> {
        println!("Flatten {:?}", path);
        let out = vec![];
        out
    }
}
