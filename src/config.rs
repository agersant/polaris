use regex;
use std::fs;
use std::io::Read;
use std::path;
use toml;

use collection::User;
use ddns::DDNSConfig;
use errors::*;
use index::IndexConfig;
use utils;
use vfs::VfsConfig;

const DEFAULT_CONFIG_FILE_NAME: &'static str = "polaris.toml";
const INDEX_FILE_NAME: &'static str = "index.sqlite";
const CONFIG_SECRET: &'static str = "auth_secret";
const CONFIG_MOUNT_DIRS: &'static str = "mount_dirs";
const CONFIG_MOUNT_DIR_NAME: &'static str = "name";
const CONFIG_MOUNT_DIR_SOURCE: &'static str = "source";
const CONFIG_USERS: &'static str = "users";
const CONFIG_USER_NAME: &'static str = "name";
const CONFIG_USER_PASSWORD: &'static str = "password";
const CONFIG_ALBUM_ART_PATTERN: &'static str = "album_art_pattern";
const CONFIG_INDEX_SLEEP_DURATION: &'static str = "reindex_every_n_seconds";
const CONFIG_DDNS: &'static str = "ydns";
const CONFIG_DDNS_HOST: &'static str = "host";
const CONFIG_DDNS_USERNAME: &'static str = "username";
const CONFIG_DDNS_PASSWORD: &'static str = "password";

pub struct Config {
    pub secret: String,
    pub vfs: VfsConfig,
    pub users: Vec<User>,
    pub index: IndexConfig,
    pub ddns: Option<DDNSConfig>,
}

impl Config {
    pub fn parse(custom_path: Option<path::PathBuf>) -> Result<Config> {

        let config_path = match custom_path {
            Some(p) => p,
            None => {
                let mut root = utils::get_config_root()?;
                root.push(DEFAULT_CONFIG_FILE_NAME);
                root
            }
        };
        println!("Config file path: {}", config_path.to_string_lossy());

        let mut config_file = fs::File::open(config_path)?;
        let mut config_file_content = String::new();
        config_file.read_to_string(&mut config_file_content)?;
        let parsed_config = toml::Parser::new(config_file_content.as_str()).parse();
        let parsed_config = parsed_config.ok_or("Could not parse config as valid TOML")?;

        let mut config = Config {
            secret: String::new(),
            vfs: VfsConfig::new(),
            users: Vec::new(),
            index: IndexConfig::new(),
            ddns: None,
        };

        config.parse_secret(&parsed_config)?;
        config.parse_index_sleep_duration(&parsed_config)?;
        config.parse_mount_points(&parsed_config)?;
        config.parse_users(&parsed_config)?;
        config.parse_album_art_pattern(&parsed_config)?;
        config.parse_ddns(&parsed_config)?;

        let mut index_path = utils::get_cache_root()?;
        index_path.push(INDEX_FILE_NAME);
        config.index.path = index_path;

        Ok(config)
    }

    fn parse_secret(&mut self, source: &toml::Table) -> Result<()> {
        self.secret = source.get(CONFIG_SECRET)
            .and_then(|s| s.as_str())
            .map(|s| s.to_owned())
            .ok_or("Could not parse config secret")?;
        Ok(())
    }

    fn parse_index_sleep_duration(&mut self, source: &toml::Table) -> Result<()> {
        let sleep_duration = match source.get(CONFIG_INDEX_SLEEP_DURATION) {
            Some(s) => s,
            None => return Ok(()),
        };
        let sleep_duration = match sleep_duration {
            &toml::Value::Integer(s) => s as u64,
            _ => bail!("Could not parse index sleep duration"),
        };
        self.index.sleep_duration = sleep_duration;
        Ok(())
    }

    fn parse_album_art_pattern(&mut self, source: &toml::Table) -> Result<()> {
        let pattern = match source.get(CONFIG_ALBUM_ART_PATTERN) {
            Some(s) => s,
            None => return Ok(()),
        };
        let pattern = match pattern {
            &toml::Value::String(ref s) => s,
            _ => bail!("Could not parse album art pattern"),
        };
        self.index.album_art_pattern = Some(regex::Regex::new(pattern)?);
        Ok(())
    }

    fn parse_users(&mut self, source: &toml::Table) -> Result<()> {
        let users = match source.get(CONFIG_USERS) {
            Some(s) => s,
            None => return Ok(()),
        };

        let users = match users {
            &toml::Value::Array(ref a) => a,
            _ => bail!("Could not parse users array"),
        };

        for user in users {
            let name = user.lookup(CONFIG_USER_NAME)
                .and_then(|n| n.as_str())
                .ok_or("Could not parse username")?;
            let password = user.lookup(CONFIG_USER_PASSWORD)
                .and_then(|n| n.as_str())
                .ok_or("Could not parse user password")?;
            let user = User::new(name.to_owned(), password.to_owned());
            self.users.push(user);
        }

        Ok(())
    }

    fn parse_mount_points(&mut self, source: &toml::Table) -> Result<()> {
        let mount_dirs = match source.get(CONFIG_MOUNT_DIRS) {
            Some(s) => s,
            None => return Ok(()),
        };

        let mount_dirs = match mount_dirs {
            &toml::Value::Array(ref a) => a,
            _ => bail!("Could not parse mount directories array"),
        };

        for dir in mount_dirs {
            let name = dir.lookup(CONFIG_MOUNT_DIR_NAME)
                .and_then(|n| n.as_str())
                .ok_or("Could not parse mount directory name")?;
            let source = dir.lookup(CONFIG_MOUNT_DIR_SOURCE)
                .and_then(|n| n.as_str())
                .ok_or("Could not parse mount directory source")?;
            let source = clean_path_string(source);
            if self.vfs.mount_points.contains_key(name) {
                bail!("Conflicting mount directories");
            }
            self.vfs.mount_points.insert(name.to_owned(), source);
        }

        Ok(())
    }

    fn parse_ddns(&mut self, source: &toml::Table) -> Result<()> {
        let ddns = match source.get(CONFIG_DDNS) {
            Some(s) => s,
            None => return Ok(()),
        };
        let ddns = match ddns {
            &toml::Value::Table(ref a) => a,
            _ => bail!("Could not parse DDNS settings table"),
        };

        let host =
            ddns.get(CONFIG_DDNS_HOST).and_then(|n| n.as_str()).ok_or("Could not parse DDNS host")?;
        let username = ddns.get(CONFIG_DDNS_USERNAME)
            .and_then(|n| n.as_str())
            .ok_or("Could not parse DDNS username")?;
        let password = ddns.get(CONFIG_DDNS_PASSWORD)
            .and_then(|n| n.as_str())
            .ok_or("Could not parse DDNS password")?;

        self.ddns = Some(DDNSConfig {
            host: host.to_owned(),
            username: username.to_owned(),
            password: password.to_owned(),
        });
        Ok(())
    }
}

fn clean_path_string(path_string: &str) -> path::PathBuf {
    let separator_regex = regex::Regex::new(r"\\|/").unwrap();
    let mut correct_separator = String::new();
    correct_separator.push(path::MAIN_SEPARATOR);
    let path_string = separator_regex.replace_all(path_string, correct_separator.as_str());
    path::PathBuf::from(path_string)
}

#[test]
fn test_clean_path_string() {
    let mut correct_path = path::PathBuf::new();
    correct_path.push("C:\\");
    correct_path.push("some");
    correct_path.push("path");
    assert_eq!(correct_path, clean_path_string(r#"C:/some/path"#));
    assert_eq!(correct_path, clean_path_string(r#"C:\some\path"#));
    assert_eq!(correct_path, clean_path_string(r#"C:\some\path\"#));
}
