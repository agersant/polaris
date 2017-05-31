use ape;
use core;
use id3;
use image;
use hyper;
use iron::IronError;
use iron::status::Status;
use lewton;
use metaflac;
use regex;
use sqlite;
use std;
use toml;

error_chain! {
    foreign_links {
        Ape(ape::Error);
        Encoding(core::str::Utf8Error);
        Flac(metaflac::Error);
        Hyper(hyper::Error);
        Id3(id3::Error);
        Image(image::ImageError);
        Io(std::io::Error);
        Time(std::time::SystemTimeError);
        Toml(toml::de::Error);
        Regex(regex::Error);
        SQLite(sqlite::Error);
        Vorbis(lewton::VorbisError);
    }

    errors {
        DaemonError {}
        AuthenticationRequired {}
        MissingUsername {}
        MissingPassword {}
        IncorrectCredentials {}
        CannotServeDirectory {}
        UnsupportedFileType {}
        MissingIndexVersion {}
    }
}

impl From<Error> for IronError {
	fn from(err: Error) -> IronError {
		match err {
			e @ Error(ErrorKind::AuthenticationRequired, _) => {
				IronError::new(e, Status::Unauthorized)
			}
			e @ Error(ErrorKind::MissingUsername, _) => IronError::new(e, Status::BadRequest),
			e @ Error(ErrorKind::MissingPassword, _) => IronError::new(e, Status::BadRequest),
			e @ Error(ErrorKind::IncorrectCredentials, _) => IronError::new(e, Status::BadRequest),
			e @ Error(ErrorKind::CannotServeDirectory, _) => IronError::new(e, Status::BadRequest),
			e @ Error(ErrorKind::UnsupportedFileType, _) => IronError::new(e, Status::BadRequest),
			e => IronError::new(e, Status::InternalServerError),
		}
	}
}
