use regex::Regex;
use std::fs;
use std::io::Read;
use std::path;
use toml;

use collection::User;
use ddns::DDNSConfig;
use errors::*;
use db::IndexConfig;
use utils;
use vfs::VfsConfig;

const DEFAULT_CONFIG_FILE_NAME: &'static str = "polaris.toml";
const INDEX_FILE_NAME: &'static str = "index.sqlite";

#[derive(Deserialize)]
struct MountDir {
	pub name: String,
	pub source: String,
}

#[derive(Deserialize)]
struct UserConfig {
	pub auth_secret: String,
	pub album_art_pattern: Option<String>,
	pub reindex_every_n_seconds: Option<u64>,
	pub mount_dirs: Vec<MountDir>,
	pub users: Vec<User>,
	pub ydns: Option<DDNSConfig>,
}

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

		// Parse user config
		let mut config_file = fs::File::open(config_path)?;
		let mut config_file_content = String::new();
		config_file.read_to_string(&mut config_file_content)?;
		let user_config = toml::de::from_str::<UserConfig>(config_file_content.as_str())?;

		// Init VFS config
		let mut vfs_config = VfsConfig::new();
		for dir in user_config.mount_dirs {
			if vfs_config.mount_points.contains_key(&dir.name) {
				bail!("Conflicting mount directories");
			}
			vfs_config
				.mount_points
				.insert(dir.name.to_owned(), clean_path_string(dir.source.as_str()));
		}

		// Init Index config
		let mut index_config = IndexConfig::new();
		index_config.album_art_pattern = user_config
			.album_art_pattern
			.and_then(|s| Regex::new(s.as_str()).ok());
		if let Some(duration) = user_config.reindex_every_n_seconds {
			index_config.sleep_duration = duration;
		}
		let mut index_path = utils::get_data_root()?;
		index_path.push(INDEX_FILE_NAME);
		index_config.path = index_path;

		// Init master config
		let config = Config {
			secret: user_config.auth_secret,
			vfs: vfs_config,
			users: user_config.users,
			index: index_config,
			ddns: user_config.ydns,
		};

		Ok(config)
	}
}

fn clean_path_string(path_string: &str) -> path::PathBuf {
	let separator_regex = Regex::new(r"\\|/").unwrap();
	let mut correct_separator = String::new();
	correct_separator.push(path::MAIN_SEPARATOR);
	let path_string = separator_regex.replace_all(path_string, correct_separator.as_str());
	path::Path::new(&path_string).iter().collect()
}

#[test]
fn test_clean_path_string() {
	let mut correct_path = path::PathBuf::new();
	if cfg!(target_os = "windows") {
		correct_path.push("C:\\");
	} else {
		correct_path.push("/usr");
	}
	correct_path.push("some");
	correct_path.push("path");
	if cfg!(target_os = "windows") {
		assert_eq!(correct_path, clean_path_string(r#"C:/some/path"#));
		assert_eq!(correct_path, clean_path_string(r#"C:\some\path"#));
		assert_eq!(correct_path, clean_path_string(r#"C:\some\path\"#));
		assert_eq!(correct_path, clean_path_string(r#"C:\some\path\\\\"#));
		assert_eq!(correct_path, clean_path_string(r#"C:\some/path//"#));
	} else {
		assert_eq!(correct_path, clean_path_string(r#"/usr/some/path"#));
		assert_eq!(correct_path, clean_path_string(r#"/usr\some\path"#));
		assert_eq!(correct_path, clean_path_string(r#"/usr\some\path\"#));
		assert_eq!(correct_path, clean_path_string(r#"/usr\some\path\\\\"#));
		assert_eq!(correct_path, clean_path_string(r#"/usr\some/path//"#));
	}

}
