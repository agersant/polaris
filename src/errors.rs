use ape;
use core;
use diesel;
use diesel_migrations;
use getopts;
use id3;
use image;
use lewton;
use metaflac;
use regex;
use rocket;
use rustfm_scrobble;
use serde_json;
use std;
use toml;

error_chain! {
	foreign_links {
		Ape(ape::Error);
		Diesel(diesel::result::Error);
		DieselConnection(diesel::ConnectionError);
		DieselMigration(diesel_migrations::RunMigrationsError);
		Encoding(core::str::Utf8Error);
		Flac(metaflac::Error);
		GetOpts(getopts::Fail);
		Id3(id3::Error);
		Image(image::ImageError);
		Io(std::io::Error);
		Json(serde_json::Error);
		Time(std::time::SystemTimeError);
		Toml(toml::de::Error);
		Regex(regex::Error);
		Scrobbler(rustfm_scrobble::ScrobblerError);
		Vorbis(lewton::VorbisError);
	}

	errors {
		DaemonError {}
		AuthenticationRequired {}
		AdminPrivilegeRequired {}
		MissingConfig {}
		MissingPreferences {}
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
		MissingLastFMCredentials {}
		LastFMAuthError {}
		LastFMDeserializationError {}
		MissingDesiredResponse {}
	}
}

impl<'r> rocket::response::Responder<'r> for Error {
	fn respond_to(self, _: &rocket::request::Request) -> rocket::response::Result<'r> {
		let mut build = rocket::response::Response::build();
		build
			.status(match self.0 {
				ErrorKind::FileNotFound => rocket::http::Status::NotFound,
				_ => rocket::http::Status::InternalServerError,
			})
			.ok()
	}
}
