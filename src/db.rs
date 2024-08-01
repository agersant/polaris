use std::path::Path;

use sqlx::{
	migrate::Migrator,
	pool::PoolConnection,
	sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqliteSynchronous},
	Sqlite,
};

use crate::app::Error;

static MIGRATOR: Migrator = sqlx::migrate!("src/db");

#[derive(Clone)]
pub struct DB {
	pool: SqlitePool,
}

impl DB {
	pub async fn new(path: &Path) -> Result<DB, Error> {
		let directory = path.parent().unwrap();
		std::fs::create_dir_all(directory).map_err(|e| Error::Io(directory.to_owned(), e))?;

		let pool = SqlitePool::connect_lazy_with(
			SqliteConnectOptions::new()
				.create_if_missing(true)
				.filename(path)
				.journal_mode(SqliteJournalMode::Wal)
				.synchronous(SqliteSynchronous::Normal),
		);

		let db = DB { pool };
		db.migrate_up().await?;
		Ok(db)
	}

	pub async fn connect(&self) -> Result<PoolConnection<Sqlite>, Error> {
		self.pool.acquire().await.map_err(|_| Error::ConnectionPool)
	}

	async fn migrate_up(&self) -> Result<(), Error> {
		MIGRATOR
			.run(&self.pool)
			.await
			.and(Ok(()))
			.map_err(Error::Migration)
	}
}

#[tokio::test]
async fn run_migrations() {
	use crate::test::*;
	use crate::test_name;
	let output_dir = prepare_test_directory(test_name!());
	let db_path = output_dir.join("db.sqlite");
	let db = DB::new(&db_path).await.unwrap();
	db.migrate_up().await.unwrap();
}
