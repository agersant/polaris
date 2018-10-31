use core::ops::Deref;
use diesel::prelude::*;
use reqwest;
use std::io;
use std::thread;
use std::time;

use crate::db::ddns_config;
use crate::db::{ConnectionSource, DB};
use crate::errors;

#[derive(Clone, Debug, Deserialize, Insertable, PartialEq, Queryable, Serialize)]
#[table_name = "ddns_config"]
pub struct DDNSConfig {
	pub host: String,
	pub username: String,
	pub password: String,
}

pub trait DDNSConfigSource {
	fn get_ddns_config(&self) -> errors::Result<DDNSConfig>;
}

impl DDNSConfigSource for DB {
	fn get_ddns_config(&self) -> errors::Result<DDNSConfig> {
		use self::ddns_config::dsl::*;
		let connection = self.get_connection();
		Ok(ddns_config
			.select((host, username, password))
			.get_result(connection.deref())?)
	}
}

#[derive(Debug)]
enum DDNSError {
	Internal(errors::Error),
	Io(io::Error),
	Reqwest(reqwest::Error),
	Update(reqwest::StatusCode),
}

impl From<io::Error> for DDNSError {
	fn from(err: io::Error) -> DDNSError {
		DDNSError::Io(err)
	}
}

impl From<errors::Error> for DDNSError {
	fn from(err: errors::Error) -> DDNSError {
		DDNSError::Internal(err)
	}
}

impl From<reqwest::Error> for DDNSError {
	fn from(err: reqwest::Error) -> DDNSError {
		DDNSError::Reqwest(err)
	}
}

const DDNS_UPDATE_URL: &str = "https://ydns.io/api/v1/update/";

fn update_my_ip<T>(config_source: &T) -> Result<(), DDNSError>
where
	T: DDNSConfigSource,
{
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
		return Err(DDNSError::Update(res.status()));
	}
	Ok(())
}

pub fn run<T>(config_source: &T)
where
	T: DDNSConfigSource,
{
	loop {
		if let Err(e) = update_my_ip(config_source) {
			error!("Dynamic DNS update error: {:?}", e);
		}
		thread::sleep(time::Duration::from_secs(60 * 30));
	}
}
