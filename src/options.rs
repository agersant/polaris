use simplelog::LevelFilter;
use std::{
	net::{AddrParseError, IpAddr},
	path::PathBuf,
	str::FromStr,
};

pub struct CLIOptions {
	pub show_help: bool,
	pub foreground: bool,
	pub log_file_path: Option<PathBuf>,
	#[cfg(unix)]
	pub pid_file_path: Option<PathBuf>,
	pub config_file_path: Option<PathBuf>,
	pub cache_dir_path: Option<PathBuf>,
	pub data_dir_path: Option<PathBuf>,
	pub web_dir_path: Option<PathBuf>,
	pub bind_address: Option<IpAddr>,
	pub port: Option<u16>,
	pub log_level: Option<LevelFilter>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error(transparent)]
	Format(getopts::Fail),
	#[error("`{0}` is not a valid bind address: {1}")]
	BindAddress(String, AddrParseError),
	#[error("`{0}` is not a valid log level. Valid log levels are `off`, `error`, `warn`, `info`, `debug` and `trace`.")]
	LogLevel(String),
}

pub struct Manager {
	protocol: getopts::Options,
}

impl Manager {
	pub fn new() -> Self {
		Self {
			protocol: get_options(),
		}
	}

	pub fn parse(&self, input: &[String]) -> Result<CLIOptions, Error> {
		let matches = self.protocol.parse(input).map_err(Error::Format)?;

		let bind_address = matches.opt_str("bind-address");
		let bind_address = bind_address
			.as_ref()
			.map(|s| IpAddr::from_str(s))
			.transpose()
			.map_err(|e| Error::BindAddress(bind_address.unwrap_or_default(), e))?;

		let log_level = matches.opt_str("log-level");
		let log_level: Option<LevelFilter> = log_level
			.as_ref()
			.map(|s| s.parse())
			.transpose()
			.or(Err(Error::LogLevel(log_level.unwrap_or_default())))?;

		Ok(CLIOptions {
			show_help: matches.opt_present("h"),
			#[cfg(unix)]
			foreground: matches.opt_present("f"),
			#[cfg(windows)]
			foreground: !cfg!(feature = "ui"),
			log_level,
			log_file_path: matches.opt_str("log").map(PathBuf::from),
			#[cfg(unix)]
			pid_file_path: matches.opt_str("pid").map(PathBuf::from),
			config_file_path: matches.opt_str("c").map(PathBuf::from),
			web_dir_path: matches.opt_str("w").map(PathBuf::from),
			cache_dir_path: matches.opt_str("cache").map(PathBuf::from),
			data_dir_path: matches.opt_str("data").map(PathBuf::from),
			bind_address,
			port: matches.opt_str("p").and_then(|p| p.parse().ok()),
		})
	}

	pub fn usage(&self, brief: &str) -> String {
		self.protocol.usage(brief)
	}
}

fn get_options() -> getopts::Options {
	let mut options = getopts::Options::new();

	options.optflag("h", "help", "print this help menu");

	#[cfg(unix)]
	options.optflag(
		"f",
		"foreground",
		"run polaris in the foreground instead of daemonizing",
	);

	options.optopt(
		"",
		"log-level",
		"set log level (off/error/warn/info/debug/trace)",
		"LEVEL",
	);

	options.optopt("", "log", "set path to log file", "FILE");
	options.optopt("", "pid", "set path to pid file", "FILE");
	options.optopt("c", "config", "set path to configuration file", "FILE");
	options.optopt(
		"w",
		"web",
		"set directory to serve as web client",
		"DIRECTORY",
	);
	options.optopt("", "cache", "set directory to use as cache", "DIRECTORY");
	options.optopt(
		"",
		"data",
		"set directory where persistent data is saved",
		"DIRECTORY",
	);

	options.optopt(
		"",
		"bind-address",
		"bind TCP listener to the specified address",
		"IP",
	);
	options.optopt(
		"p",
		"port",
		"bind TCP listener to the specified port",
		"PORT",
	);

	options
}
