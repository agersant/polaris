use std::error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum PError {
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

impl From<io::Error> for PError {
    fn from(err: io::Error) -> PError {
        PError::Io(err)
    }
}

impl error::Error for PError {
    fn description(&self) -> &str {
        match *self {
            PError::Io(ref err) => err.description(),
            PError::PathDecoding => "Error while decoding a Path as a UTF-8 string",
            PError::ConflictingMount => {
                "Attempting to mount multiple directories under the same name"
            }
            PError::PathNotInVfs => "Requested path does not index a mount point",
            PError::CannotServeDirectory => "Only individual files can be served",
            PError::ConfigFileOpenError => "Could not open config file",
            PError::ConfigFileReadError => "Could not read config file",
            PError::ConfigFileParseError => "Could not parse config file",
            PError::ConfigMountDirsParseError => "Could not parse mount directories in config file",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            PError::Io(ref err) => Some(err),
            PError::PathDecoding => None,
            PError::ConflictingMount => None,
            PError::PathNotInVfs => None,
            PError::CannotServeDirectory => None,
            PError::ConfigFileOpenError => None,
            PError::ConfigFileReadError => None,
            PError::ConfigFileParseError => None,
            PError::ConfigMountDirsParseError => None,
        }
    }
}

impl fmt::Display for PError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            PError::Io(ref err) => write!(f, "IO error: {}", err),
            PError::PathDecoding => write!(f, "Path decoding error"),
            PError::ConflictingMount => {
                write!(f, "Mount point already has a target directory")
            }
            PError::PathNotInVfs => {
                write!(f, "Requested path does not index a mount point")
            }
            PError::CannotServeDirectory => {
                write!(f, "Only individual files can be served")
            }
            PError::ConfigFileOpenError => {
                write!(f, "Could not open config file")
            }
            PError::ConfigFileReadError => {
                write!(f, "Could not read config file")
            }
            PError::ConfigFileParseError => {
                write!(f, "Could not parse config file")
            }
            PError::ConfigMountDirsParseError => {
                write!(f, "Could not parse mount directories in config file")
            }
        }
    }
}
