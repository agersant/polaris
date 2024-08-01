use regex::Regex;
use serde::Deserialize;
use std::time::Duration;

use crate::app::Error;
use crate::db::DB;

#[derive(Clone, Default)]
pub struct AuthSecret {
	pub key: [u8; 32],
}

#[derive(Debug)]
pub struct Settings {
	pub index_sleep_duration_seconds: i64,
	pub index_album_art_pattern: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct NewSettings {
	pub reindex_every_n_seconds: Option<i64>,
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

	pub async fn get_auth_secret(&self) -> Result<AuthSecret, Error> {
		sqlx::query_scalar!("SELECT auth_secret FROM config")
			.fetch_one(self.db.connect().await?.as_mut())
			.await?
			.try_into()
			.map_err(|_| Error::AuthenticationSecretInvalid)
			.map(|key| AuthSecret { key })
	}

	pub async fn get_index_sleep_duration(&self) -> Result<Duration, Error> {
		let settings = self.read().await?;
		Ok(Duration::from_secs(
			settings.index_sleep_duration_seconds as u64,
		))
	}

	pub async fn get_index_album_art_pattern(&self) -> Result<Regex, Error> {
		let settings = self.read().await?;
		let regex = Regex::new(&format!("(?i){}", &settings.index_album_art_pattern))
			.map_err(|_| Error::IndexAlbumArtPatternInvalid)?;
		Ok(regex)
	}

	pub async fn read(&self) -> Result<Settings, Error> {
		Ok(sqlx::query_as!(
			Settings,
			"SELECT index_sleep_duration_seconds,index_album_art_pattern FROM config"
		)
		.fetch_one(self.db.connect().await?.as_mut())
		.await?)
	}

	pub async fn amend(&self, new_settings: &NewSettings) -> Result<(), Error> {
		let mut connection = self.db.connect().await?;

		if let Some(sleep_duration) = new_settings.reindex_every_n_seconds {
			sqlx::query!(
				"UPDATE config SET index_sleep_duration_seconds = $1",
				sleep_duration
			)
			.execute(connection.as_mut())
			.await?;
		}

		if let Some(ref album_art_pattern) = new_settings.album_art_pattern {
			sqlx::query!(
				"UPDATE config SET index_album_art_pattern = $1",
				album_art_pattern
			)
			.execute(connection.as_mut())
			.await?;
		}

		Ok(())
	}
}
