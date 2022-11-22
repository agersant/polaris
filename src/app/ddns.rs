use diesel::prelude::*;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::thread;
use std::time;

use crate::db::{self, ddns_config, DB};

const DDNS_UPDATE_URL: &str = "https://ydns.io/api/v1/update/";

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("DDNS update query failed with HTTP status code `{0}`")]
	UpdateQueryFailed(u16),
	#[error(transparent)]
	DatabaseConnection(#[from] db::Error),
	#[error(transparent)]
	Database(#[from] diesel::result::Error),
}

#[derive(Clone, Debug, Deserialize, Insertable, PartialEq, Eq, Queryable, Serialize)]
#[diesel(table_name = ddns_config)]
pub struct Config {
	pub host: String,
	pub username: String,
	pub password: String,
}

#[derive(Clone)]
pub struct Manager {
	db: DB,
}

impl Manager {
	pub fn new(db: DB) -> Self {
		Self { db }
	}

	fn update_my_ip(&self) -> Result<(), Error> {
		let config = self.config()?;
		if config.host.is_empty() || config.username.is_empty() {
			info!("Skipping DDNS update because credentials are missing");
			return Ok(());
		}

		let full_url = format!("{}?host={}", DDNS_UPDATE_URL, &config.host);
		let response = ureq::get(full_url.as_str())
			.auth(&config.username, &config.password)
			.call();

		if response.ok() {
			Ok(())
		} else {
			Err(Error::UpdateQueryFailed(response.status()))
		}
	}

	pub fn config(&self) -> Result<Config, Error> {
		use crate::db::ddns_config::dsl::*;
		let mut connection = self.db.connect()?;
		Ok(ddns_config
			.select((host, username, password))
			.get_result(&mut connection)?)
	}

	pub fn set_config(&self, new_config: &Config) -> Result<(), Error> {
		use crate::db::ddns_config::dsl::*;
		let mut connection = self.db.connect()?;
		diesel::update(ddns_config)
			.set((
				host.eq(&new_config.host),
				username.eq(&new_config.username),
				password.eq(&new_config.password),
			))
			.execute(&mut connection)?;
		Ok(())
	}

	pub fn begin_periodic_updates(&self) {
		let cloned = self.clone();
		std::thread::spawn(move || {
			cloned.run();
		});
	}

	fn run(&self) {
		loop {
			if let Err(e) = self.update_my_ip() {
				error!("Dynamic DNS update error: {:?}", e);
			}
			thread::sleep(time::Duration::from_secs(60 * 30));
		}
	}
}
