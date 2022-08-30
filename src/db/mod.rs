use anyhow::{bail, Error, Result};
use diesel::r2d2::{self, ConnectionManager, PooledConnection};
use diesel::sqlite::SqliteConnection;
use diesel::RunQueryDsl;
use diesel_migrations::EmbeddedMigrations;
use diesel_migrations::MigrationHarness;
use std::path::Path;

mod schema;

pub use self::schema::*;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

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
	pub fn new(path: &Path) -> Result<DB> {
		std::fs::create_dir_all(&path.parent().unwrap())?;
		let manager = ConnectionManager::<SqliteConnection>::new(path.to_string_lossy());
		let pool = diesel::r2d2::Pool::builder()
			.connection_customizer(Box::new(ConnectionCustomizer {}))
			.build(manager)?;
		let db = DB { pool };
		db.migrate_up()?;
		Ok(db)
	}

	pub fn connect(&self) -> Result<PooledConnection<ConnectionManager<SqliteConnection>>> {
		self.pool.get().map_err(Error::new)
	}

	#[allow(dead_code)]
	fn migrate_down(&self) -> Result<()> {
		let mut connection = self.connect().unwrap();
		if let Err(e) = connection.revert_all_migrations(MIGRATIONS) {
			bail!(e);
		}
		Ok(())
	}

	fn migrate_up(&self) -> Result<()> {
		let mut connection = self.connect().unwrap();
		connection.run_pending_migrations(MIGRATIONS).unwrap();
		Ok(())
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
