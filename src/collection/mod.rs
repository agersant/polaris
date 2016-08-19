use std::fs;
use std::path::Path;

pub use self::error::CollectionError; 

mod error;

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

pub fn browse(path: &Path) -> Result<Vec<CollectionFile>, CollectionError> {

    let full_path = "samplemusic/".to_string() + path.to_str().unwrap(); // TMP use mount directories
    println!("Browsing: {}", full_path);

    let mut out = vec![];
    for file in try!(fs::read_dir(full_path)) {
        let file = try!(file);
        let file_meta = try!(file.metadata());
        let file_path = file.path().to_owned();
        if file_meta.is_file() {
            let path_string = try!(file_path.to_str().ok_or(CollectionError::PathDecoding)); 
            let collection_file = CollectionFile::Song(Song {
                path: path_string.to_string(),
            });
            out.push(collection_file);
        } else if file_meta.is_dir() {
            let path_string = try!(file_path.to_str().ok_or(CollectionError::PathDecoding)); 
            let collection_file = CollectionFile::Directory(Directory {
                path: path_string.to_string(),
            });
            out.push(collection_file);
        }
    }

    Ok(out)
}

pub fn flatten(path: &Path) -> Vec<CollectionFile> {
    println!("Flatten {:?}", path);
    let out = vec![];
    out
}
