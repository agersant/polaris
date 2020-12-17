use diesel;
use diesel::prelude::*;
use regex::Regex;
use std::convert::TryInto;
use std::time::Duration;

use super::*;
use crate::db::{ddns_config, misc_settings, mount_points, DB};

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
		let connection = self.db.connect()?;
		let secret: Vec<u8> = misc_settings
			.select(auth_secret)
			.get_result(&connection)
			.map_err(|e| match e {
				diesel::result::Error::NotFound => Error::AuthSecretNotFound,
				_ => Error::Unspecified,
			})?;
		secret
			.try_into()
			.map_err(|_| Error::InvalidAuthSecret)
			.map(|key| AuthSecret { key })
	}

	pub fn get_index_sleep_duration(&self) -> Result<Duration, Error> {
		use self::misc_settings::dsl::*;
		let connection = self.db.connect()?;
		misc_settings
			.select(index_sleep_duration_seconds)
			.get_result(&connection)
			.map_err(|e| match e {
				diesel::result::Error::NotFound => Error::IndexSleepDurationNotFound,
				_ => Error::Unspecified,
			})
			.map(|s: i32| Duration::from_secs(s as u64))
	}

	pub fn get_index_album_art_pattern(&self) -> Result<Regex, Error> {
		use self::misc_settings::dsl::*;
		let connection = self.db.connect()?;
		misc_settings
			.select(index_album_art_pattern)
			.get_result(&connection)
			.map_err(|e| match e {
				diesel::result::Error::NotFound => Error::IndexAlbumArtPatternNotFound,
				_ => Error::Unspecified,
			})
			.and_then(|s: String| {
				Regex::new(&format!("(?i){}", &s)).map_err(|_| Error::IndexAlbumArtPatternInvalid)
			})
	}

	pub fn read(&self) -> Result<Settings, Error> {
		let connection = self.db.connect()?;

		let misc: MiscSettings = misc_settings::table
			.get_result(&connection)
			.map_err(|_| Error::Unspecified)?;

		let mount_dirs = {
			use self::mount_points::dsl::*;
			mount_points
				.select((source, name))
				.get_results(&connection)
				.map_err(|_| Error::Unspecified)?
		};

		let ydns = ddns_config::table
			.select((
				ddns_config::host,
				ddns_config::username,
				ddns_config::password,
			))
			.get_result(&connection)
			.ok();

		Ok(Settings {
			auth_secret: misc.auth_secret,
			index_album_art_pattern: misc.index_album_art_pattern,
			index_sleep_duration_seconds: misc.index_sleep_duration_seconds,
			mount_dirs,
			ydns,
		})
	}

	pub fn amend(&self, new_settings: &NewSettings) -> Result<(), Error> {
		let connection = self.db.connect()?;

		if let Some(ref mount_dirs) = new_settings.mount_dirs {
			diesel::delete(mount_points::table)
				.execute(&connection)
				.map_err(|_| Error::Unspecified)?;
			diesel::insert_into(mount_points::table)
				.values(mount_dirs)
				.execute(&*connection)
				.map_err(|_| Error::Unspecified)?; // TODO https://github.com/diesel-rs/diesel/issues/1822
		}

		if let Some(sleep_duration) = new_settings.index_sleep_duration_seconds {
			diesel::update(misc_settings::table)
				.set(misc_settings::index_sleep_duration_seconds.eq(sleep_duration as i32))
				.execute(&connection)
				.map_err(|_| Error::Unspecified)?;
		}

		if let Some(ref album_art_pattern) = new_settings.index_album_art_pattern {
			diesel::update(misc_settings::table)
				.set(misc_settings::index_album_art_pattern.eq(album_art_pattern))
				.execute(&connection)
				.map_err(|_| Error::Unspecified)?;
		}

		if let Some(ref ydns) = new_settings.ydns {
			use self::ddns_config::dsl::*;
			diesel::update(ddns_config)
				.set((
					host.eq(ydns.host.clone()),
					username.eq(ydns.username.clone()),
					password.eq(ydns.password.clone()),
				))
				.execute(&connection)
				.map_err(|_| Error::Unspecified)?;
		}

		Ok(())
	}
}
