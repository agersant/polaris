use std::error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum SwineError {
    PathDecoding,
    Io(io::Error),
    ConflictingMount,
    PathNotInVfs,
    CannotServeDirectory,
    ConfigFileOpenError,
    ConfigFileReadError,
    ConfigFileParseError,
    ConfigMountDirsParseError,
}

impl From<io::Error> for SwineError {
    fn from(err: io::Error) -> SwineError {
        SwineError::Io(err)
    }
}

impl error::Error for SwineError {
    fn description(&self) -> &str {
        match *self {
            SwineError::Io(ref err) => err.description(),
            SwineError::PathDecoding => "Error while decoding a Path as a UTF-8 string",
            SwineError::ConflictingMount => {
                "Attempting to mount multiple directories under the same name"
            }
            SwineError::PathNotInVfs => "Requested path does not index a mount point",
            SwineError::CannotServeDirectory => "Only individual files can be served",
            SwineError::ConfigFileOpenError => "Could not open config file",
            SwineError::ConfigFileReadError => "Could not read config file",
            SwineError::ConfigFileParseError => "Could not parse config file",
            SwineError::ConfigMountDirsParseError => "Could not parse mount directories in config file",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            SwineError::Io(ref err) => Some(err),
            SwineError::PathDecoding => None,
            SwineError::ConflictingMount => None,
            SwineError::PathNotInVfs => None,
            SwineError::CannotServeDirectory => None,
            SwineError::ConfigFileOpenError => None,
            SwineError::ConfigFileReadError => None,
            SwineError::ConfigFileParseError => None,
            SwineError::ConfigMountDirsParseError => None,
        }
    }
}

impl fmt::Display for SwineError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SwineError::Io(ref err) => write!(f, "IO error: {}", err),
            SwineError::PathDecoding => write!(f, "Path decoding error"),
            SwineError::ConflictingMount => {
                write!(f, "Mount point already has a target directory")
            }
            SwineError::PathNotInVfs => {
                write!(f, "Requested path does not index a mount point")
            }
            SwineError::CannotServeDirectory => {
                write!(f, "Only individual files can be served")
            }
            SwineError::ConfigFileOpenError => {
                write!(f, "Could not open config file")
            }
            SwineError::ConfigFileReadError => {
                write!(f, "Could not read config file")
            }
            SwineError::ConfigFileParseError => {
                write!(f, "Could not parse config file")
            }
            SwineError::ConfigMountDirsParseError => {
                write!(f, "Could not parse mount directories in config file")
            }
        }
    }
}
