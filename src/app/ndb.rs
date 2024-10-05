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
	pub fn new(path: &Path) -> Result<Self, Error> {
		let database = native_db::Builder::new()
			.create(&MODELS, path)
			.map_err(|e| Error::NativeDatabaseCreationError(e))?;
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
