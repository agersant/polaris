use std::error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum CollectionError
{
    PathDecoding,
    Io(io::Error),
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
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            CollectionError::Io(ref err) => Some(err),
            CollectionError::PathDecoding => None,
        }
    }
}

impl fmt::Display for CollectionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CollectionError::Io(ref err) => write!(f, "IO error: {}", err),
            CollectionError::PathDecoding => write!(f, "Path decoding error"),
        }
    }
}