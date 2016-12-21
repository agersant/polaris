use hyper;
use hyper::client::Client;
use hyper::header::{Authorization, Basic};
use std::io;
use std::io::Read;
use std::thread;
use std::time;

#[derive(Debug, Clone)]
pub struct DDNSConfig {
	pub host: String,
	pub username: String,
	pub password: String,
}

#[derive(Debug)]
enum DDNSError {
	IoError(io::Error),
	HyperError(hyper::Error),
	UpdateError(hyper::status::StatusCode),
}

impl From<io::Error> for DDNSError {
	fn from(err: io::Error) -> DDNSError {
		DDNSError::IoError(err)
	}
}

impl From<hyper::Error> for DDNSError {
	fn from(err: hyper::Error) -> DDNSError {
		DDNSError::HyperError(err)
	}
}

const MY_IP_API_URL: &'static str = "http://api.ipify.org";
const DDNS_UPDATE_URL: &'static str = "http://ydns.io/api/v1/update/";

fn get_my_ip() -> Result<String, DDNSError> {
	let client = Client::new();
	let mut res = client.get(MY_IP_API_URL).send()?;
	let mut buf = String::new();
	res.read_to_string(&mut buf)?;
	Ok(buf)
}

fn update_my_ip(ip: &String, config: &DDNSConfig) -> Result<(), DDNSError> {
	let client = Client::new();
	let url = DDNS_UPDATE_URL;
	let host = &config.host;
	let full_url = format!("{}?host={}&ip={}", url, host, ip);
	let auth_header = Authorization(Basic {
		username: config.username.clone(),
		password: Some(config.password.to_owned()),
	});

	let res = client.get(full_url.as_str()).header(auth_header).send()?;
	match res.status {
		hyper::status::StatusCode::Ok => Ok(()),
		s => Err(DDNSError::UpdateError(s)),
	}
}

pub fn run(config: DDNSConfig) {
	loop {
		let my_ip_res = get_my_ip();
		if let Ok(my_ip) = my_ip_res {
			match update_my_ip(&my_ip, &config) {
				Err(e) => println!("Dynamic DNS Error: {:?}", e),
				Ok(_) => (),
			};
		} else {
			println!("Dynamic DNS Error: could not retrieve our own IP address");
		}
		thread::sleep(time::Duration::from_secs(60 * 30));
	}
}
