use base64::prelude::*;
use log::{debug, error};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::db::{self, DB};

const DDNS_UPDATE_URL: &str = "https://ydns.io/api/v1/update/";

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("DDNS update query failed with HTTP status code `{0}`")]
	UpdateQueryFailed(u16),
	#[error("DDNS update query failed due to a transport error")]
	UpdateQueryTransport,
	#[error(transparent)]
	DatabaseConnection(#[from] db::Error),
	#[error(transparent)]
	Database(#[from] sqlx::Error),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Config {
	pub ddns_host: String,
	pub ddns_username: String,
	pub ddns_password: String,
}

#[derive(Clone)]
pub struct Manager {
	db: DB,
}

impl Manager {
	pub fn new(db: DB) -> Self {
		Self { db }
	}

	async fn update_my_ip(&self) -> Result<(), Error> {
		let config = self.config().await?;
		if config.ddns_host.is_empty() || config.ddns_username.is_empty() {
			debug!("Skipping DDNS update because credentials are missing");
			return Ok(());
		}

		let full_url = format!("{}?host={}", DDNS_UPDATE_URL, &config.ddns_host);
		let credentials = format!("{}:{}", &config.ddns_username, &config.ddns_password);
		let response = ureq::get(full_url.as_str())
			.set(
				"Authorization",
				&format!("Basic {}", BASE64_STANDARD_NO_PAD.encode(credentials)),
			)
			.call();

		match response {
			Ok(_) => Ok(()),
			Err(ureq::Error::Status(code, _)) => Err(Error::UpdateQueryFailed(code)),
			Err(ureq::Error::Transport(_)) => Err(Error::UpdateQueryTransport),
		}
	}

	pub async fn config(&self) -> Result<Config, Error> {
		Ok(sqlx::query_as!(
			Config,
			"SELECT ddns_host, ddns_username, ddns_password FROM config"
		)
		.fetch_one(self.db.connect().await?.as_mut())
		.await?)
	}

	pub async fn set_config(&self, new_config: &Config) -> Result<(), Error> {
		sqlx::query!(
			"UPDATE config SET ddns_host = $1, ddns_username = $2, ddns_password = $3",
			new_config.ddns_host,
			new_config.ddns_username,
			new_config.ddns_password
		)
		.execute(self.db.connect().await?.as_mut())
		.await?;
		Ok(())
	}

	pub fn begin_periodic_updates(&self) {
		tokio::spawn({
			let ddns = self.clone();
			async move {
				loop {
					if let Err(e) = ddns.update_my_ip().await {
						error!("Dynamic DNS update error: {:?}", e);
					}
					tokio::time::sleep(Duration::from_secs(60 * 30)).await;
				}
			}
		});
	}
}
