use core::ops::Deref;
use diesel;
use diesel::expression::sql;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel::types;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use config::UserConfig;
use db::schema::*;
use ddns::{DDNSConfigSource, DDNSConfig};
use errors::*;
use vfs::Vfs;

mod index;
mod models;
mod schema;

pub use self::index::Index;
pub use self::models::*;

#[allow(dead_code)]
const DB_MIGRATIONS_PATH: &'static str = "src/db/migrations";
embed_migrations!("src/db/migrations");

pub struct DB {
	connection: Arc<Mutex<SqliteConnection>>,
	index: Index,
}

impl DB {
	pub fn new(path: &Path) -> Result<DB> {
		println!("Database file path: {}", path.to_string_lossy());
		let connection =
			Arc::new(Mutex::new(SqliteConnection::establish(&path.to_string_lossy())?));
		let db = DB {
			connection: connection.clone(),
			index: Index::new(),
		};
		db.init()?;
		Ok(db)
	}

	fn init(&self) -> Result<()> {
		{
			let connection = self.connection.lock().unwrap();
			connection.execute("PRAGMA synchronous = NORMAL")?;
		}
		self.migrate_up()?;
		Ok(())
	}

	pub fn get_connection(&self) -> Arc<Mutex<SqliteConnection>> {
		self.connection.clone()
	}

	pub fn get_index(&self) -> &Index {
		&self.index
	}

	#[allow(dead_code)]
	fn migrate_down(&self) -> Result<()> {
		let connection = self.connection.lock().unwrap();
		let connection = connection.deref();
		loop {
			match diesel::migrations::revert_latest_migration_in_directory(connection, Path::new(DB_MIGRATIONS_PATH)) {
				Ok(_) => (),
				Err(diesel::migrations::RunMigrationsError::MigrationError(diesel::migrations::MigrationError::NoMigrationRun)) => break,
				Err(e) => bail!(e),
			}
		}
		Ok(())
	}

	fn migrate_up(&self) -> Result<()> {
		let connection = self.connection.lock().unwrap();
		let connection = connection.deref();
		embedded_migrations::run(connection)?;
		Ok(())
	}

	pub fn load_config(&self, config: &UserConfig) -> Result<()> {
		let connection = self.connection.lock().unwrap();
		let connection = connection.deref();
		if let Some(ref mount_dirs) = config.mount_dirs {
			diesel::delete(mount_points::table).execute(connection)?;
			diesel::insert(mount_dirs)
				.into(mount_points::table)
				.execute(connection)?;
		}

		if let Some(ref config_users) = config.users {
			diesel::delete(users::table).execute(connection)?;
			for config_user in config_users {
				let new_user = NewUser::new(&config_user.name, &config_user.password);
				println!("new user: {}", &config_user.name);
				diesel::insert(&new_user)
					.into(users::table)
					.execute(connection)?;
			}
		}

		if let Some(sleep_duration) = config.reindex_every_n_seconds {
			diesel::update(misc_settings::table)
				.set(misc_settings::index_sleep_duration_seconds.eq(sleep_duration as i32))
				.execute(connection)?;
		}

		if let Some(ref album_art_pattern) = config.album_art_pattern {
			diesel::update(misc_settings::table)
				.set(misc_settings::index_album_art_pattern.eq(album_art_pattern))
				.execute(connection)?;
		}

		Ok(())
	}

	pub fn get_auth_secret(&self) -> Result<String> {
		let connection = self.connection.lock().unwrap();
		let connection = connection.deref();
		let misc: MiscSettings = misc_settings::table.get_result(connection)?;
		Ok(misc.auth_secret.to_owned())
	}

	pub fn locate(&self, virtual_path: &Path) -> Result<PathBuf> {
		let vfs = self.get_vfs()?;
		vfs.virtual_to_real(virtual_path)
	}

	fn get_vfs(&self) -> Result<Vfs> {
		let mut vfs = Vfs::new();
		let connection = self.connection.lock().unwrap();
		let connection = connection.deref();
		let mount_points: Vec<MountPoint> = mount_points::table.get_results(connection)?;
		for mount_point in mount_points {
			vfs.mount(&Path::new(&mount_point.real_path), &mount_point.name)?;
		}
		Ok(vfs)
	}

	fn virtualize_song(&self, vfs: &Vfs, mut song: Song) -> Option<Song> {
		song.path = match vfs.real_to_virtual(Path::new(&song.path)) {
			Ok(p) => p.to_string_lossy().into_owned(),
			_ => return None,
		};
		if let Some(artwork_path) = song.artwork {
			song.artwork = match vfs.real_to_virtual(Path::new(&artwork_path)) {
				Ok(p) => Some(p.to_string_lossy().into_owned()),
				_ => None,
			};
		}
		Some(song)
	}

	fn virtualize_directory(&self, vfs: &Vfs, mut directory: Directory) -> Option<Directory> {
		directory.path = match vfs.real_to_virtual(Path::new(&directory.path)) {
			Ok(p) => p.to_string_lossy().into_owned(),
			_ => return None,
		};
		if let Some(artwork_path) = directory.artwork {
			directory.artwork = match vfs.real_to_virtual(Path::new(&artwork_path)) {
				Ok(p) => Some(p.to_string_lossy().into_owned()),
				_ => None,
			};
		}
		Some(directory)
	}

	pub fn browse(&self, virtual_path: &Path) -> Result<Vec<CollectionFile>> {
		let mut output = Vec::new();
		let vfs = self.get_vfs()?;
		let connection = self.connection.lock().unwrap();
		let connection = connection.deref();

		if virtual_path.components().count() == 0 {
			// Browse top-level
			let real_directories: Vec<Directory> = directories::table
				.filter(directories::parent.is_null())
				.load(connection)?;
			let virtual_directories = real_directories
				.into_iter()
				.filter_map(|s| self.virtualize_directory(&vfs, s));
			output.extend(virtual_directories
			                  .into_iter()
			                  .map(|d| CollectionFile::Directory(d)));

		} else {
			// Browse sub-directory
			let real_path = vfs.virtual_to_real(virtual_path)?;
			let real_path_string = real_path.as_path().to_string_lossy().into_owned();

			let real_directories: Vec<Directory> = directories::table
				.filter(directories::parent.eq(&real_path_string))
				.order(directories::path)
				.load(connection)?;
			let virtual_directories = real_directories
				.into_iter()
				.filter_map(|s| self.virtualize_directory(&vfs, s));
			output.extend(virtual_directories.map(|d| CollectionFile::Directory(d)));

			let real_songs: Vec<Song> = songs::table
				.filter(songs::parent.eq(&real_path_string))
				.order(songs::path)
				.load(connection)?;
			let virtual_songs = real_songs
				.into_iter()
				.filter_map(|s| self.virtualize_song(&vfs, s));
			output.extend(virtual_songs.map(|s| CollectionFile::Song(s)));
		}

		Ok(output)
	}

	pub fn flatten(&self, virtual_path: &Path) -> Result<Vec<Song>> {
		use self::songs::dsl::*;
		let vfs = self.get_vfs()?;
		let connection = self.connection.lock().unwrap();
		let connection = connection.deref();
		let real_path = vfs.virtual_to_real(virtual_path)?;
		let like_path = real_path.as_path().to_string_lossy().into_owned() + "%";
		let real_songs: Vec<Song> = songs.filter(path.like(&like_path)).load(connection)?;
		let virtual_songs = real_songs
			.into_iter()
			.filter_map(|s| self.virtualize_song(&vfs, s));
		Ok(virtual_songs.collect::<Vec<_>>())
	}

	pub fn get_random_albums(&self, count: i64) -> Result<Vec<Directory>> {
		use self::directories::dsl::*;
		let vfs = self.get_vfs()?;
		let connection = self.connection.lock().unwrap();
		let connection = connection.deref();
		let real_directories = directories
			.filter(album.is_not_null())
			.limit(count)
			.order(sql::<types::Bool>("RANDOM()"))
			.load(connection)?;
		let virtual_directories = real_directories
			.into_iter()
			.filter_map(|s| self.virtualize_directory(&vfs, s));
		Ok(virtual_directories.collect::<Vec<_>>())
	}

	pub fn get_recent_albums(&self, count: i64) -> Result<Vec<Directory>> {
		use self::directories::dsl::*;
		let vfs = self.get_vfs()?;
		let connection = self.connection.lock().unwrap();
		let connection = connection.deref();
		let real_directories: Vec<Directory> = directories
			.filter(album.is_not_null())
			.order(date_added.desc())
			.limit(count)
			.load(connection)?;
		let virtual_directories = real_directories
			.into_iter()
			.filter_map(|s| self.virtualize_directory(&vfs, s));
		Ok(virtual_directories.collect::<Vec<_>>())
	}

	pub fn auth(&self, username: &str, password: &str) -> Result<bool> {
		use self::users::dsl::*;
		let connection = self.connection.lock().unwrap();
		let connection = connection.deref();
		let user: User = users.filter(name.eq(username)).get_result(connection)?;
		Ok(user.verify_password(password))
	}
}

impl DDNSConfigSource for DB {
	fn get_ddns_config(&self) -> Result<DDNSConfig> {
		use self::ddns_config::dsl::*;
		let connection = self.connection.lock().unwrap();
		let connection = connection.deref();
		Ok(ddns_config
		       .select((host, username, password))
		       .get_result(connection)?)
	}
}

fn _get_test_db(name: &str) -> DB {
	let config_path = Path::new("test/config.toml");
	let config = UserConfig::parse(&config_path).unwrap();

	let mut db_path = PathBuf::new();
	db_path.push("test");
	db_path.push(name);
	if db_path.exists() {
		fs::remove_file(&db_path).unwrap();
	}

	let db = DB::new(&db_path).unwrap();
	db.load_config(&config).unwrap();
	db
}

#[test]
fn test_migrations_up() {
	_get_test_db("migrations_up.sqlite");
}

#[test]
fn test_migrations_down() {
	let db = _get_test_db("migrations_down.sqlite");
	db.migrate_down().unwrap();
	db.migrate_up().unwrap();
}

#[test]
fn test_browse_top_level() {
	let mut root_path = PathBuf::new();
	root_path.push("root");

	let db = _get_test_db("browse_top_level.sqlite");
	db.get_index().update_index(&db).unwrap();
	let results = db.browse(Path::new("")).unwrap();

	assert_eq!(results.len(), 1);
	match results[0] {
		CollectionFile::Directory(ref d) => assert_eq!(d.path, root_path.to_str().unwrap()),
		_ => panic!("Expected directory"),
	}
}

#[test]
fn test_browse() {
	let mut khemmis_path = PathBuf::new();
	khemmis_path.push("root");
	khemmis_path.push("Khemmis");

	let mut tobokegao_path = PathBuf::new();
	tobokegao_path.push("root");
	tobokegao_path.push("Tobokegao");

	let db = _get_test_db("browse.sqlite");
	db.get_index().update_index(&db).unwrap();
	let results = db.browse(Path::new("root")).unwrap();

	assert_eq!(results.len(), 2);
	match results[0] {
		CollectionFile::Directory(ref d) => assert_eq!(d.path, khemmis_path.to_str().unwrap()),
		_ => panic!("Expected directory"),
	}
	match results[1] {
		CollectionFile::Directory(ref d) => assert_eq!(d.path, tobokegao_path.to_str().unwrap()),
		_ => panic!("Expected directory"),
	}
}

#[test]
fn test_flatten() {
	let db = _get_test_db("flatten.sqlite");
	db.get_index().update_index(&db).unwrap();
	let results = db.flatten(Path::new("root")).unwrap();
	assert_eq!(results.len(), 12);
}

#[test]
fn test_random() {
	let db = _get_test_db("random.sqlite");
	db.get_index().update_index(&db).unwrap();
	let results = db.get_random_albums(1).unwrap();
	assert_eq!(results.len(), 1);
}

#[test]
fn test_recent() {
	let db = _get_test_db("recent.sqlite");
	db.get_index().update_index(&db).unwrap();
	let results = db.get_recent_albums(2).unwrap();
	assert_eq!(results.len(), 2);
	assert!(results[0].date_added >= results[1].date_added);
}
