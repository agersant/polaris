use core::ops::Deref;
use diesel;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use config::{MiscSettings, UserConfig};
use ddns::{DDNSConfigSource, DDNSConfig};
use errors::*;
use index;
use user::*;
use vfs::{MountPoint, VFS, VFSSource};

mod schema;

pub use self::schema::*;

#[allow(dead_code)]
const DB_MIGRATIONS_PATH: &'static str = "src/db/migrations";
embed_migrations!("src/db/migrations");

pub trait ConnectionSource {
	fn get_connection(&self) -> Arc<Mutex<SqliteConnection>>;
}

pub struct DB {
	connection: Arc<Mutex<SqliteConnection>>,
}

impl DB {
	pub fn new(path: &Path) -> Result<DB> {
		println!("Database file path: {}", path.to_string_lossy());
		let connection =
			Arc::new(Mutex::new(SqliteConnection::establish(&path.to_string_lossy())?));
		let db = DB { connection: connection.clone() };
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

	pub fn index_update(&self) -> Result<()> {
		index::update(self)
	}

	pub fn index_update_loop(&self) {
		index::update_loop(self);
	}
}

impl ConnectionSource for DB {
	fn get_connection(&self) -> Arc<Mutex<SqliteConnection>> {
		self.connection.clone()
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

impl VFSSource for DB {
	fn get_vfs(&self) -> Result<VFS> {
		use self::mount_points::dsl::*;
		let mut vfs = VFS::new();
		let connection = self.connection.lock().unwrap();
		let connection = connection.deref();
		let points: Vec<MountPoint> = mount_points
			.select((source, name))
			.get_results(connection)?;
		for point in points {
			vfs.mount(&Path::new(&point.source), &point.name)?;
		}
		Ok(vfs)
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
