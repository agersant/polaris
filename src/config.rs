use regex;
use std::fs;
use std::io;
use std::io::Read;
use std::path;
use toml;

use collection::User;
use ddns::DDNSConfig;
use vfs::VfsConfig;

const CONFIG_SECRET: &'static str = "auth_secret";
const CONFIG_MOUNT_DIRS: &'static str = "mount_dirs";
const CONFIG_MOUNT_DIR_NAME: &'static str = "name";
const CONFIG_MOUNT_DIR_SOURCE: &'static str = "source";
const CONFIG_USERS: &'static str = "users";
const CONFIG_USER_NAME: &'static str = "name";
const CONFIG_USER_PASSWORD: &'static str = "password";
const CONFIG_ALBUM_ART_PATTERN: &'static str = "album_art_pattern";
const CONFIG_DDNS: &'static str = "ydns";
const CONFIG_DDNS_HOST: &'static str = "host";
const CONFIG_DDNS_USERNAME: &'static str = "username";
const CONFIG_DDNS_PASSWORD: &'static str = "password";

#[derive(Debug)]
pub enum ConfigError {
	IoError(io::Error),
	TOMLParseError,
	RegexError(regex::Error),
	SecretParseError,
	AlbumArtPatternParseError,
	UsersParseError,
	MountDirsParseError,
	DDNSParseError,
    ConflictingMounts,
}

impl From<io::Error> for ConfigError {
    fn from(err: io::Error) -> ConfigError {
        ConfigError::IoError(err)
    }
}

impl From<regex::Error> for ConfigError {
    fn from(err: regex::Error) -> ConfigError {
        ConfigError::RegexError(err)
    }
}

pub struct Config {
	pub secret: String,
	pub vfs: VfsConfig,
    pub users: Vec<User>,
    pub album_art_pattern: Option<regex::Regex>,
	pub ddns: Option<DDNSConfig>,
}

impl Config {
	pub fn parse(config_path: &path::Path) -> Result<Config, ConfigError> {
		let mut config_file = try!(fs::File::open(config_path));
        let mut config_file_content = String::new();
		try!(config_file.read_to_string(&mut config_file_content));
		let parsed_config = toml::Parser::new(config_file_content.as_str()).parse();
        let parsed_config = try!(parsed_config.ok_or(ConfigError::TOMLParseError));

		let mut config = Config {
			secret: String::new(),
			vfs: VfsConfig::new(),
			users: Vec::new(),
			album_art_pattern: None,
			ddns: None,
		};

		try!(config.parse_secret(&parsed_config));
		try!(config.parse_mount_points(&parsed_config));
        try!(config.parse_users(&parsed_config));
        try!(config.parse_album_art_pattern(&parsed_config));
        try!(config.parse_ddns(&parsed_config));

		Ok(config)
	}

	fn parse_secret(&mut self, source: &toml::Table) -> Result<(), ConfigError> {
		let secret = try!(source.get(CONFIG_SECRET).ok_or(ConfigError::SecretParseError));
		let secret = try!(secret.as_str().ok_or(ConfigError::SecretParseError));
		self.secret = secret.to_owned();
		Ok(())
	}

	fn parse_album_art_pattern(&mut self, source: &toml::Table) -> Result<(), ConfigError> {
        let pattern = match source.get(CONFIG_ALBUM_ART_PATTERN) {
            Some(s) => s,
            None => return Ok(()),
        };
        let pattern = match pattern {
            &toml::Value::String(ref s) => s,
            _ => return Err(ConfigError::AlbumArtPatternParseError),
        };
        self.album_art_pattern = Some(try!(regex::Regex::new(pattern)));
        Ok(())
    }

	fn parse_users(&mut self, source: &toml::Table) -> Result<(), ConfigError> {
        let users = match source.get(CONFIG_USERS) {
            Some(s) => s,
            None => return Ok(()),
        };

        let users = match users {
            &toml::Value::Array(ref a) => a,
            _ => return Err(ConfigError::UsersParseError),
        };

        for user in users {
            let name = match user.lookup(CONFIG_USER_NAME) {
                None => return Err(ConfigError::UsersParseError),
                Some(n) => n,
            };
            let name = match name.as_str() {
                None => return Err(ConfigError::UsersParseError),
                Some(n) => n,
            };

            let password = match user.lookup(CONFIG_USER_PASSWORD) {
                None => return Err(ConfigError::UsersParseError),
                Some(n) => n,
            };
            let password = match password.as_str() {
                None => return Err(ConfigError::UsersParseError),
                Some(n) => n,
            };

            let user = User::new(name.to_owned(), password.to_owned());
            self.users.push(user);
        }

        Ok(())
    }

	fn parse_mount_points(&mut self, source: &toml::Table) -> Result<(), ConfigError> {
        let mount_dirs = match source.get(CONFIG_MOUNT_DIRS) {
            Some(s) => s,
            None => return Ok(()),
        };

        let mount_dirs = match mount_dirs {
            &toml::Value::Array(ref a) => a,
            _ => return Err(ConfigError::MountDirsParseError),
        };

        for dir in mount_dirs {
            let name = match dir.lookup(CONFIG_MOUNT_DIR_NAME) {
                None => return Err(ConfigError::MountDirsParseError),
                Some(n) => n,
            };
            let name = match name.as_str() {
                None => return Err(ConfigError::MountDirsParseError),
                Some(n) => n,
            };

            let source = match dir.lookup(CONFIG_MOUNT_DIR_SOURCE) {
                None => return Err(ConfigError::MountDirsParseError),
                Some(n) => n,
            };
            let source = match source.as_str() {
                None => return Err(ConfigError::MountDirsParseError),
                Some(n) => n,
            };
            let source = clean_path_string(source);

            if self.vfs.mount_points.contains_key(name) {
                return Err(ConfigError::ConflictingMounts);
            }
            self.vfs.mount_points.insert(name.to_owned(), source);
        }

        Ok(())
    }

	fn parse_ddns(&mut self, source: &toml::Table) -> Result<(), ConfigError> {
        let ddns = match source.get(CONFIG_DDNS) {
            Some(s) => s,
            None => return Ok(()),
        };
		let ddns = match ddns {
            &toml::Value::Table(ref a) => a,
            _ => return Err(ConfigError::DDNSParseError),
        };

		let host = try!(ddns.get(CONFIG_DDNS_HOST).ok_or(ConfigError::DDNSParseError)).as_str();
		let username = try!(ddns.get(CONFIG_DDNS_USERNAME).ok_or(ConfigError::DDNSParseError)).as_str();
		let password = try!(ddns.get(CONFIG_DDNS_PASSWORD).ok_or(ConfigError::DDNSParseError)).as_str();

		let host = try!(host.ok_or(ConfigError::DDNSParseError)); 
		let username = try!(username.ok_or(ConfigError::DDNSParseError)); 
		let password = try!(password.ok_or(ConfigError::DDNSParseError)); 

		self.ddns = Some(DDNSConfig {
			host: host.to_owned(),
			username: username.to_owned(),
			password: password.to_owned(),
		});
		Ok(())
    }
}

fn clean_path_string(path_string: &str) -> path::PathBuf {
    let separator = regex::Regex::new(r"\\|/").unwrap();
    let components = separator.split(path_string).collect::<Vec<_>>();
    let mut path = path::PathBuf::new();
    for component in components {
        path.push(component);
    }
    path
}

#[test]
fn test_clean_path_string() {
    let mut correct_path = path::PathBuf::new();
    correct_path.push("C:"); 
    correct_path.push("some"); 
    correct_path.push("path");
    assert_eq!(correct_path, clean_path_string(r#"C:/some/path"#)); 
    assert_eq!(correct_path, clean_path_string(r#"C:\some\path"#)); 
    assert_eq!(correct_path, clean_path_string(r#"C:\some\path\"#)); 
}
