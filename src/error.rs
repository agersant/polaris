use std::error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum CollectionError {
    PathDecoding,
    Io(io::Error),
    ConflictingMount,
    PathNotInVfs,
}

impl From<io::Error> for CollectionError {
    fn from(err: io::Error) -> CollectionError {
        CollectionError::Io(err)
    }
}

impl error::Error for CollectionError {
    fn description(&self) -> &str {
        match *self {
            CollectionError::Io(ref err) => err.description(),
            CollectionError::PathDecoding => "Error while decoding a Path as a UTF-8 string",
            CollectionError::ConflictingMount => {
                "Attempting to mount multiple directories under the same name"
            }
            CollectionError::PathNotInVfs => "Requested path does not index a mount point",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            CollectionError::Io(ref err) => Some(err),
            CollectionError::PathDecoding => None,
            CollectionError::ConflictingMount => None,
            CollectionError::PathNotInVfs => None,
        }
    }
}

impl fmt::Display for CollectionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CollectionError::Io(ref err) => write!(f, "IO error: {}", err),
            CollectionError::PathDecoding => write!(f, "Path decoding error"),
            CollectionError::ConflictingMount => {
                write!(f, "Mount point already has a target directory")
            }
            CollectionError::PathNotInVfs => {
                write!(f, "Requested path does not index a mount point")
            }
        }
    }
}
