use ape;
use core;
use diesel;
use id3;
use mp3_metadata;
use getopts;
use image;
use hyper;
use iron::IronError;
use iron::status::Status;
use lewton;
use metaflac;
use regex;
use serde_json;
use std;
use toml;

error_chain! {
    foreign_links {
        Ape(ape::Error);
        Diesel(diesel::result::Error);
        DieselConnection(diesel::ConnectionError);
        DieselMigration(diesel::migrations::RunMigrationsError);
        Encoding(core::str::Utf8Error);
        Flac(metaflac::Error);
        GetOpts(getopts::Fail);
        Hyper(hyper::Error);
        Id3(id3::Error);
		Mp3MetaData(mp3_metadata::Error);
        Image(image::ImageError);
        Io(std::io::Error);
        Json(serde_json::Error);
        Time(std::time::SystemTimeError);
        Toml(toml::de::Error);
        Regex(regex::Error);
        Vorbis(lewton::VorbisError);
    }

    errors {
        DaemonError {}
        AuthenticationRequired {}
        AdminPrivilegeRequired {}
        MissingConfig {}
        MissingUsername {}
        MissingPassword {}
        MissingPlaylist {}
        IncorrectCredentials {}
        CannotServeDirectory {}
        UnsupportedFileType {}
        FileNotFound {}
        MissingIndexVersion {}
        MissingPlaylistName {}
        EncodingError {}
    }
}

impl From<Error> for IronError {
	fn from(err: Error) -> IronError {
		match err {
			e @ Error(ErrorKind::AuthenticationRequired, _) => {
				IronError::new(e, Status::Unauthorized)
			}
			e @ Error(ErrorKind::AdminPrivilegeRequired, _) => IronError::new(e, Status::Forbidden),
			e @ Error(ErrorKind::MissingUsername, _) => IronError::new(e, Status::BadRequest),
			e @ Error(ErrorKind::MissingPassword, _) => IronError::new(e, Status::BadRequest),
			e @ Error(ErrorKind::IncorrectCredentials, _) => {
				IronError::new(e, Status::Unauthorized)
			}
			e @ Error(ErrorKind::CannotServeDirectory, _) => IronError::new(e, Status::BadRequest),
			e @ Error(ErrorKind::UnsupportedFileType, _) => IronError::new(e, Status::BadRequest),
			e => IronError::new(e, Status::InternalServerError),
		}
	}
}
