use anyhow::*;
use diesel::prelude::*;
use log::{error, info};
use reqwest;
use std::thread;
use std::time;

use super::*;
use crate::db::DB;

const DDNS_UPDATE_URL: &str = "https://ydns.io/api/v1/update/";

pub struct Manager {
	db: DB,
}

impl Manager {
	pub fn new(db: DB) -> Self {
		Self { db }
	}

	fn update_my_ip(&self) -> Result<()> {
		let config = self.get_config()?;
		if config.host.is_empty() || config.username.is_empty() {
			info!("Skipping DDNS update because credentials are missing");
			return Ok(());
		}

		let full_url = format!("{}?host={}", DDNS_UPDATE_URL, &config.host);
		let client = reqwest::ClientBuilder::new().build()?;
		let response = client
			.get(full_url.as_str())
			.basic_auth(config.username, Some(config.password))
			.send()?;

		if !response.status().is_success() {
			bail!(
				"DDNS update query failed with status code: {}",
				response.status()
			);
		}
		Ok(())
	}

	fn get_config(&self) -> Result<Config> {
		use crate::db::ddns_config::dsl::*;
		let connection = self.db.connect()?;
		Ok(ddns_config
			.select((host, username, password))
			.get_result(&connection)?)
	}

	pub fn run(&self) {
		loop {
			if let Err(e) = self.update_my_ip() {
				error!("Dynamic DNS update error: {:?}", e);
			}
			thread::sleep(time::Duration::from_secs(60 * 30));
		}
	}
}
