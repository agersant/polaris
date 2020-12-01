use anyhow::Result;
use simplelog::LevelFilter;
use std::path::PathBuf;

pub struct CLIOptions {
	pub show_help: bool,
	#[cfg(unix)]
	pub foreground: bool,
	pub log_file_path: Option<PathBuf>,
	pub pid_file_path: Option<PathBuf>,
	pub config_file_path: Option<PathBuf>,
	pub database_file_path: Option<PathBuf>,
	pub cache_dir_path: Option<PathBuf>,
	pub web_dir_path: Option<PathBuf>,
	pub swagger_dir_path: Option<PathBuf>,
	pub port: Option<u16>,
	pub log_level: Option<LevelFilter>,
}

pub struct OptionsManager {
	protocol: getopts::Options,
}

impl OptionsManager {
	pub fn new() -> Self {
		Self {
			protocol: get_options(),
		}
	}

	pub fn parse(&self, input: &[String]) -> Result<CLIOptions> {
		let matches = self.protocol.parse(input)?;

		Ok(CLIOptions {
			show_help: matches.opt_present("h"),
			#[cfg(unix)]
			foreground: matches.opt_present("f"),
			log_file_path: matches.opt_str("log").map(PathBuf::from),
			pid_file_path: matches.opt_str("pid").map(PathBuf::from),
			config_file_path: matches.opt_str("c").map(PathBuf::from),
			database_file_path: matches.opt_str("d").map(PathBuf::from),
			cache_dir_path: matches.opt_str("cache").map(PathBuf::from),
			web_dir_path: matches.opt_str("w").map(PathBuf::from),
			swagger_dir_path: matches.opt_str("s").map(PathBuf::from),
			port: matches.opt_str("p").and_then(|p| p.parse().ok()),
			log_level: matches.opt_str("log-level").and_then(|l| l.parse().ok()),
		})
	}

	pub fn usage(&self, brief: &str) -> String {
		self.protocol.usage(brief)
	}
}

fn get_options() -> getopts::Options {
	let mut options = getopts::Options::new();
	options.optopt("c", "config", "set the configuration file", "FILE");
	options.optopt("p", "port", "set polaris to run on a custom port", "PORT");
	options.optopt("d", "database", "set the path to index database", "FILE");
	options.optopt("w", "web", "set the path to web client files", "DIRECTORY");
	options.optopt("s", "swagger", "set the path to swagger files", "DIRECTORY");
	options.optopt(
		"",
		"cache",
		"set the directory to use as cache",
		"DIRECTORY",
	);
	options.optopt("", "log", "set the path to the log file", "FILE");
	options.optopt("", "pid", "set the path to the pid file", "FILE");
	options.optopt(
		"",
		"log-level",
		"set the log level to a value between 0 (off) and 3 (debug)",
		"LEVEL",
	);

	#[cfg(unix)]
	options.optflag(
		"f",
		"foreground",
		"run polaris in the foreground instead of daemonizing",
	);

	options.optflag("h", "help", "print this help menu");
	options
}
