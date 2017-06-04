use reqwest;
use reqwest::header::{Authorization, Basic};
use std::io;
use std::thread;
use std::time;

#[derive(Clone, Debug, Deserialize)]
pub struct DDNSConfig {
	pub host: String,
	pub username: String,
	pub password: String,
}

#[derive(Debug)]
enum DDNSError {
	IoError(io::Error),
	ReqwestError(reqwest::Error),
	UpdateError(reqwest::StatusCode),
}

impl From<io::Error> for DDNSError {
	fn from(err: io::Error) -> DDNSError {
		DDNSError::IoError(err)
	}
}

impl From<reqwest::Error> for DDNSError {
	fn from(err: reqwest::Error) -> DDNSError {
		DDNSError::ReqwestError(err)
	}
}

const DDNS_UPDATE_URL: &'static str = "https://ydns.io/api/v1/update/";


fn update_my_ip(config: &DDNSConfig) -> Result<(), DDNSError> {
	let full_url = format!("{}?host={}", DDNS_UPDATE_URL, &config.host);
	let auth_header = Authorization(Basic {
	                                    username: config.username.clone(),
	                                    password: Some(config.password.to_owned()),
	                                });
	let client = reqwest::Client::new()?;
	let res = client
		.get(full_url.as_str())
		.header(auth_header)
		.send()?;
	if !res.status().is_success() {
		return Err(DDNSError::UpdateError(*res.status()));
	}
	Ok(())
}

pub fn run(config: DDNSConfig) {
	loop {
		match update_my_ip(&config) {
			Err(e) => println!("Dynamic DNS Error: {:?}", e),
			Ok(_) => (),
		};
		thread::sleep(time::Duration::from_secs(60 * 30));
	}
}
