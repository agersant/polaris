use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;


#[derive(Debug, RustcEncodable)]
pub struct Song(PathBuf);

#[derive(Debug, RustcEncodable)]
pub struct Directory(PathBuf);

#[derive(Debug, RustcEncodable)]
pub enum CollectionFile {
    Directory(Directory),
    Song(Song),
}

pub enum CollectionError
{
    Io(io::Error),
}

impl From<io::Error> for CollectionError {
    fn from(err: io::Error) -> CollectionError {
        CollectionError::Io(err)
    }
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
            let collection_file = CollectionFile::Song(Song(file_path));
            out.push(collection_file);
        } else if file_meta.is_dir() {
            let collection_file = CollectionFile::Directory(Directory(file_path));
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
