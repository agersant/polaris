use reqwest;
use reqwest::header::{Authorization, Basic};
use std::io;
use std::thread;
use std::time;

use db::DB;
use errors;


#[derive(Debug)]
enum DDNSError {
	InternalError(errors::Error),
	IoError(io::Error),
	ReqwestError(reqwest::Error),
	UpdateError(reqwest::StatusCode),
}

impl From<io::Error> for DDNSError {
	fn from(err: io::Error) -> DDNSError {
		DDNSError::IoError(err)
	}
}

impl From<errors::Error> for DDNSError {
	fn from(err: errors::Error) -> DDNSError {
		DDNSError::InternalError(err)
	}
}

impl From<reqwest::Error> for DDNSError {
	fn from(err: reqwest::Error) -> DDNSError {
		DDNSError::ReqwestError(err)
	}
}

const DDNS_UPDATE_URL: &'static str = "https://ydns.io/api/v1/update/";


fn update_my_ip(db: &DB) -> Result<(), DDNSError> {
	let config = db.get_ddns_config()?;
	if config.host.len() == 0 || config.username.len() == 0 {
		println!("Skipping DDNS update because credentials are missing");
		return Ok(());
	}

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

pub fn run(db: &DB) {
	loop {
		match update_my_ip(db) {
			Err(e) => println!("Dynamic DNS Error: {:?}", e),
			Ok(_) => (),
		};
		thread::sleep(time::Duration::from_secs(60 * 30));
	}
}
