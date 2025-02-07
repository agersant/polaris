use std::{
	ops::Deref,
	path::Path,
	sync::{Arc, LazyLock},
};

use native_db::{Database, Models};

use crate::app::{playlist, Error};

static MODELS: LazyLock<Models> = LazyLock::new(|| {
	let mut models = Models::new();
	models.define::<playlist::v1::PlaylistModel>().unwrap();
	models
});

#[derive(Clone)]
pub struct Manager {
	database: Arc<Database<'static>>,
}

impl Manager {
	pub fn new(directory: &Path) -> Result<Self, Error> {
		std::fs::create_dir_all(directory).map_err(|e| Error::Io(directory.to_owned(), e))?;
		let path = directory.join("polaris.ndb");
		let database = native_db::Builder::new()
			.create(&MODELS, path)
			.map_err(Error::NativeDatabaseCreationError)?;
		let database = Arc::new(database);
		Ok(Self { database })
	}
}

impl Deref for Manager {
	type Target = Database<'static>;

	fn deref(&self) -> &Self::Target {
		self.database.as_ref()
	}
}
