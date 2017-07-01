use core::ops::Deref;
use diesel;
use diesel::prelude::*;
use regex::Regex;
use std::fs;
use std::io::Read;
use std::path;
use toml;

use db::ConnectionSource;
use db::{ddns_config, misc_settings, mount_points, users};
use ddns::DDNSConfig;
use errors::*;
use user::*;
use vfs::MountPoint;

#[derive(Debug, Queryable)]
pub struct MiscSettings {
	id: i32,
	pub auth_secret: String,
	pub index_sleep_duration_seconds: i32,
	pub index_album_art_pattern: String,
}

#[derive(Serialize, Deserialize)]
pub struct ConfigUser {
	pub name: String,
	pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
	pub album_art_pattern: Option<String>,
	pub reindex_every_n_seconds: Option<i32>,
	pub mount_dirs: Option<Vec<MountPoint>>,
	pub users: Option<Vec<ConfigUser>>,
	pub ydns: Option<DDNSConfig>,
}

pub fn parse(path: &path::Path) -> Result<Config> {
	println!("Config file path: {}", path.to_string_lossy());

	// Parse user config
	let mut config_file = fs::File::open(path)?;
	let mut config_file_content = String::new();
	config_file.read_to_string(&mut config_file_content)?;
	let mut config = toml::de::from_str::<Config>(config_file_content.as_str())?;

	// Clean path
	if let Some(ref mut mount_dirs) = config.mount_dirs {
		for mount_dir in mount_dirs {
			match clean_path_string(&mount_dir.source).to_str() {
				Some(p) => mount_dir.source = p.to_owned(),
				_ => bail!("Bad mount directory path"),
			}
		}
	}

	Ok(config)
}

pub fn read<T>(db: &T) -> Result<Config>
	where T: ConnectionSource
{
	use self::misc_settings::dsl::*;
	use self::mount_points::dsl::*;
	use self::ddns_config::dsl::*;

	let connection = db.get_connection();
	let connection = connection.lock().unwrap();
	let connection = connection.deref();

	let mut config = Config {
		album_art_pattern: None,
		reindex_every_n_seconds: None,
		mount_dirs: None,
		users: None,
		ydns: None,
	};

	let (art_pattern, sleep_duration) = misc_settings
		.select((index_album_art_pattern, index_sleep_duration_seconds))
		.get_result(connection)?;
	config.album_art_pattern = Some(art_pattern);
	config.reindex_every_n_seconds = Some(sleep_duration);

	let mount_dirs = mount_points
		.select((source, name))
		.get_results(connection)?;
	config.mount_dirs = Some(mount_dirs);

	let usernames: Vec<String> = users::table
		.select(users::columns::name)
		.get_results(connection)?;
	config.users = Some(usernames
	                        .into_iter()
	                        .map(|s| {
		                             ConfigUser {
		                                 name: s,
		                                 password: "".to_owned(),
		                             }
		                            })
	                        .collect::<_>());

	let ydns = ddns_config
		.select((host, username, password))
		.get_result(connection)?;
	config.ydns = Some(ydns);

	Ok(config)
}

fn reset<T>(db: &T) -> Result<()>
	where T: ConnectionSource
{
	let connection = db.get_connection();
	let connection = connection.lock().unwrap();
	let connection = connection.deref();

	diesel::delete(mount_points::table).execute(connection)?;
	diesel::delete(users::table).execute(connection)?;

	Ok(())
}

pub fn overwrite<T>(db: &T, new_config: &Config) -> Result<()>
	where T: ConnectionSource
{
	reset(db)?;
	ammend(db, new_config)
}

pub fn ammend<T>(db: &T, new_config: &Config) -> Result<()>
	where T: ConnectionSource
{
	let connection = db.get_connection();
	let connection = connection.lock().unwrap();
	let connection = connection.deref();

	if let Some(ref mount_dirs) = new_config.mount_dirs {
		diesel::insert(mount_dirs)
			.into(mount_points::table)
			.execute(connection)?;
	}

	if let Some(ref config_users) = new_config.users {
		for config_user in config_users {
			let new_user = User::new(&config_user.name, &config_user.password);
			diesel::insert(&new_user)
				.into(users::table)
				.execute(connection)?;
		}
	}

	if let Some(sleep_duration) = new_config.reindex_every_n_seconds {
		diesel::update(misc_settings::table)
			.set(misc_settings::index_sleep_duration_seconds.eq(sleep_duration as i32))
			.execute(connection)?;
	}

	if let Some(ref album_art_pattern) = new_config.album_art_pattern {
		diesel::update(misc_settings::table)
			.set(misc_settings::index_album_art_pattern.eq(album_art_pattern))
			.execute(connection)?;
	}

	Ok(())
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
