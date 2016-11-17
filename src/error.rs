use ape;
use std::error;
use std::fmt;
use std::io;
use id3;
use image;
use lewton;

#[derive(Debug)]
pub enum PError {
    CannotClearExistingIndex,
    PathDecoding,
    Io(io::Error),
    PathNotInVfs,
    CannotServeDirectory,
    UnsupportedFileType,
    ConfigFileOpenError,
    ConfigFileReadError,
    ConfigFileParseError,
    ConfigMountDirsParseError,
    ConfigUsersParseError,
    ConfigAlbumArtPatternParseError,
    AlbumArtSearchError,
    ImageProcessingError,
    UnsupportedMetadataFormat,
    APEParseError,
    ID3ParseError,
    VorbisParseError,
    Unauthorized,
    IncorrectCredentials,
}

impl From<ape::Error> for PError {
    fn from(_: ape::Error) -> PError {
        PError::APEParseError
    }
}

impl From<io::Error> for PError {
    fn from(err: io::Error) -> PError {
        PError::Io(err)
    }
}

impl From<id3::Error> for PError {
    fn from(_: id3::Error) -> PError {
        PError::ID3ParseError
    }
}

impl From<image::ImageError> for PError {
    fn from(_: image::ImageError) -> PError {
        PError::ImageProcessingError
    }
}

impl From<lewton::VorbisError> for PError {
    fn from(_: lewton::VorbisError) -> PError {
        PError::VorbisParseError
    }
}

impl error::Error for PError {
    fn description(&self) -> &str {
        match *self {
            PError::Io(ref err) => err.description(),
            PError::CannotClearExistingIndex => "Error while removing existing index",
            PError::PathDecoding => "Error while decoding a Path as a UTF-8 string",
            PError::PathNotInVfs => "Requested path does not index a mount point",
            PError::CannotServeDirectory => "Only individual files can be served",
            PError::UnsupportedFileType => "Unrecognized extension",
            PError::ConfigFileOpenError => "Could not open config file",
            PError::ConfigFileReadError => "Could not read config file",
            PError::ConfigFileParseError => "Could not parse config file",
            PError::ConfigMountDirsParseError => "Could not parse mount directories in config file",
            PError::ConfigUsersParseError => "Could not parse users in config file",
            PError::ConfigAlbumArtPatternParseError => {
                "Could not parse album art pattern in config file"
            }
            PError::AlbumArtSearchError => "Error while looking for album art",
            PError::ImageProcessingError => "Error while processing image",
            PError::UnsupportedMetadataFormat => "Unsupported metadata format",
            PError::APEParseError => "Error while reading APE tag",
            PError::ID3ParseError => "Error while reading ID3 tag",
            PError::VorbisParseError => "Error while reading Vorbis tag",
            PError::Unauthorized => "Authentication required",
            PError::IncorrectCredentials => "Incorrect username/password combination",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            PError::Io(ref err) => Some(err),
            _ => None,
        }
    }
}

impl fmt::Display for PError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            PError::Io(ref err) => write!(f, "IO error: {}", err),
            PError::CannotClearExistingIndex => write!(f, "Error while removing existing index"),
            PError::PathDecoding => write!(f, "Path decoding error"),
            PError::PathNotInVfs => write!(f, "Requested path does not index a mount point"),
            PError::CannotServeDirectory => write!(f, "Only individual files can be served"),
            PError::UnsupportedFileType => write!(f, "Unrecognized extension"),
            PError::ConfigFileOpenError => write!(f, "Could not open config file"),
            PError::ConfigFileReadError => write!(f, "Could not read config file"),
            PError::ConfigFileParseError => write!(f, "Could not parse config file"),
            PError::ConfigUsersParseError => write!(f, "Could not parse users in config file"),
            PError::ConfigMountDirsParseError => {
                write!(f, "Could not parse mount directories in config file")
            }
            PError::ConfigAlbumArtPatternParseError => {
                write!(f, "Could not album art pattern in config file")
            }
            PError::AlbumArtSearchError => write!(f, "Error while looking for album art"),
            PError::ImageProcessingError => write!(f, "Error while processing image"),
            PError::UnsupportedMetadataFormat => write!(f, "Unsupported metadata format"),
            PError::APEParseError => write!(f, "Error while reading APE tag"),
            PError::ID3ParseError => write!(f, "Error while reading ID3 tag"),
            PError::VorbisParseError => write!(f, "Error while reading Vorbis tag"),
            PError::Unauthorized => write!(f, "Authentication required"),
            PError::IncorrectCredentials => write!(f, "Incorrect username/password combination"),
        }
    }
}
