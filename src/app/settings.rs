use diesel::prelude::*;
use regex::Regex;
use serde::Deserialize;
use std::convert::TryInto;
use std::time::Duration;

use crate::db::{self, misc_settings, DB};

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("Auth secret does not have the expected format")]
	AuthenticationSecretInvalid,
	#[error("Missing auth secret")]
	AuthenticationSecretNotFound,
	#[error(transparent)]
	DatabaseConnection(#[from] db::Error),
	#[error("Missing settings")]
	MiscSettingsNotFound,
	#[error("Index album art pattern is not a valid regex")]
	IndexAlbumArtPatternInvalid,
	#[error(transparent)]
	Database(#[from] diesel::result::Error),
}

#[derive(Clone, Default)]
pub struct AuthSecret {
	pub key: [u8; 32],
}

#[derive(Debug, Queryable)]
pub struct Settings {
	pub index_sleep_duration_seconds: i32,
	pub index_album_art_pattern: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct NewSettings {
	pub reindex_every_n_seconds: Option<i32>,
	pub album_art_pattern: Option<String>,
}

#[derive(Clone)]
pub struct Manager {
	pub db: DB,
}

impl Manager {
	pub fn new(db: DB) -> Self {
		Self { db }
	}

	pub fn get_auth_secret(&self) -> Result<AuthSecret, Error> {
		use self::misc_settings::dsl::*;
		let mut connection = self.db.connect()?;
		let secret: Vec<u8> = misc_settings
			.select(auth_secret)
			.get_result(&mut connection)
			.map_err(|e| match e {
				diesel::result::Error::NotFound => Error::AuthenticationSecretNotFound,
				e => e.into(),
			})?;
		secret
			.try_into()
			.map_err(|_| Error::AuthenticationSecretInvalid)
			.map(|key| AuthSecret { key })
	}

	pub fn get_index_sleep_duration(&self) -> Result<Duration, Error> {
		let settings = self.read()?;
		Ok(Duration::from_secs(
			settings.index_sleep_duration_seconds as u64,
		))
	}

	pub fn get_index_album_art_pattern(&self) -> Result<Regex, Error> {
		let settings = self.read()?;
		let regex = Regex::new(&format!("(?i){}", &settings.index_album_art_pattern))
			.map_err(|_| Error::IndexAlbumArtPatternInvalid)?;
		Ok(regex)
	}

	pub fn read(&self) -> Result<Settings, Error> {
		use self::misc_settings::dsl::*;
		let mut connection = self.db.connect()?;

		let settings: Settings = misc_settings
			.select((index_sleep_duration_seconds, index_album_art_pattern))
			.get_result(&mut connection)
			.map_err(|e| match e {
				diesel::result::Error::NotFound => Error::MiscSettingsNotFound,
				e => e.into(),
			})?;

		Ok(settings)
	}

	pub fn amend(&self, new_settings: &NewSettings) -> Result<(), Error> {
		let mut connection = self.db.connect()?;

		if let Some(sleep_duration) = new_settings.reindex_every_n_seconds {
			diesel::update(misc_settings::table)
				.set(misc_settings::index_sleep_duration_seconds.eq(sleep_duration))
				.execute(&mut connection)?;
		}

		if let Some(ref album_art_pattern) = new_settings.album_art_pattern {
			diesel::update(misc_settings::table)
				.set(misc_settings::index_album_art_pattern.eq(album_art_pattern))
				.execute(&mut connection)?;
		}

		Ok(())
	}
}
