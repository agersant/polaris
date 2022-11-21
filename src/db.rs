use diesel::r2d2::{self, ConnectionManager, PooledConnection};
use diesel::sqlite::SqliteConnection;
use diesel::RunQueryDsl;
use diesel_migrations::EmbeddedMigrations;
use diesel_migrations::MigrationHarness;
use std::path::{Path, PathBuf};

mod schema;

pub use self::schema::*;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("Could not initialize database connection pool")]
	ConnectionPoolBuild,
	#[error("Could not acquire database connection from pool")]
	ConnectionPool,
	#[error("Filesystem error for `{0}`: `{1}`")]
	Io(PathBuf, std::io::Error),
	#[error("Could not apply database migrations")]
	Migration,
}

#[derive(Clone)]
pub struct DB {
	pool: r2d2::Pool<ConnectionManager<SqliteConnection>>,
}

#[derive(Debug)]
struct ConnectionCustomizer {}
impl diesel::r2d2::CustomizeConnection<SqliteConnection, diesel::r2d2::Error>
	for ConnectionCustomizer
{
	fn on_acquire(&self, connection: &mut SqliteConnection) -> Result<(), diesel::r2d2::Error> {
		let query = diesel::sql_query(
			r#"
			PRAGMA busy_timeout = 60000;
			PRAGMA journal_mode = WAL;
			PRAGMA synchronous = NORMAL;
			PRAGMA foreign_keys = ON;
		"#,
		);
		query
			.execute(connection)
			.map_err(diesel::r2d2::Error::QueryError)?;
		Ok(())
	}
}

impl DB {
	pub fn new(path: &Path) -> Result<DB, Error> {
		let directory = path.parent().unwrap();
		std::fs::create_dir_all(directory).map_err(|e| Error::Io(directory.to_owned(), e))?;
		let manager = ConnectionManager::<SqliteConnection>::new(path.to_string_lossy());
		let pool = diesel::r2d2::Pool::builder()
			.connection_customizer(Box::new(ConnectionCustomizer {}))
			.build(manager)
			.or(Err(Error::ConnectionPoolBuild))?;
		let db = DB { pool };
		db.migrate_up()?;
		Ok(db)
	}

	pub fn connect(&self) -> Result<PooledConnection<ConnectionManager<SqliteConnection>>, Error> {
		self.pool.get().or(Err(Error::ConnectionPool))
	}

	#[cfg(test)]
	fn migrate_down(&self) -> Result<(), Error> {
		let mut connection = self.connect()?;
		connection
			.revert_all_migrations(MIGRATIONS)
			.and(Ok(()))
			.or(Err(Error::Migration))
	}

	fn migrate_up(&self) -> Result<(), Error> {
		let mut connection = self.connect()?;
		connection
			.run_pending_migrations(MIGRATIONS)
			.and(Ok(()))
			.or(Err(Error::Migration))
	}
}

#[test]
fn run_migrations() {
	use crate::test::*;
	use crate::test_name;
	let output_dir = prepare_test_directory(test_name!());
	let db_path = output_dir.join("db.sqlite");
	let db = DB::new(&db_path).unwrap();

	db.migrate_down().unwrap();
	db.migrate_up().unwrap();
}
