use anyhow::*;
use core::ops::Deref;
use diesel::prelude::*;
use log::{error, info};
use reqwest;
use serde::{Deserialize, Serialize};
use std::thread;
use std::time;

use crate::db::ddns_config;
use crate::db::DB;

#[derive(Clone, Debug, Deserialize, Insertable, PartialEq, Queryable, Serialize)]
#[table_name = "ddns_config"]
pub struct DDNSConfig {
	pub host: String,
	pub username: String,
	pub password: String,
}

pub trait DDNSConfigSource {
	fn get_ddns_config(&self) -> Result<DDNSConfig>;
}

impl DDNSConfigSource for DB {
	fn get_ddns_config(&self) -> Result<DDNSConfig> {
		use self::ddns_config::dsl::*;
		let connection = self.connect()?;
		Ok(ddns_config
			.select((host, username, password))
			.get_result(connection.deref())?)
	}
}

const DDNS_UPDATE_URL: &str = "https://ydns.io/api/v1/update/";

fn update_my_ip(config_source: &DB) -> Result<()> {
	let config = config_source.get_ddns_config()?;
	if config.host.is_empty() || config.username.is_empty() {
		info!("Skipping DDNS update because credentials are missing");
		return Ok(());
	}

	let full_url = format!("{}?host={}", DDNS_UPDATE_URL, &config.host);
	let client = reqwest::ClientBuilder::new().build()?;
	let res = client
		.get(full_url.as_str())
		.basic_auth(config.username, Some(config.password))
		.send()?;
	if !res.status().is_success() {
		bail!(
			"DDNS update query failed with status code: {}",
			res.status()
		);
	}
	Ok(())
}

pub fn run(config_source: &DB) {
	loop {
		if let Err(e) = update_my_ip(config_source) {
			error!("Dynamic DNS update error: {:?}", e);
		}
		thread::sleep(time::Duration::from_secs(60 * 30));
	}
}
