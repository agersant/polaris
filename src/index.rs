use sqlite;
use core::ops::Deref;
use std::fs;
use std::fs::DirBuilder;
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time;

use error::*;
use vfs::Vfs;

pub struct Index {
	path: String,
}

impl Index {

	pub fn new(path: &Path) -> Result<Index, PError> {

		// Create target directory
		let mut dir_path = path.to_path_buf();
		if dir_path.components().count() > 1 {
			dir_path.pop();
		}
		let mut dir_builder = DirBuilder::new();
		dir_builder.recursive(true);
		dir_builder.create(dir_path).unwrap();

		// Init Index
		let index = Index {
			path: path.to_string_lossy().deref().to_string(),
		};

		// Setup DB
		if path.exists() {
			match fs::remove_file(&index.path) {
				Err(_) => return Err(PError::CannotClearExistingIndex),
				_ => (),
			}
		}

		let db = index.connect();
		db.execute("

			CREATE TABLE artists
			(	id INTEGER PRIMARY KEY NOT NULL
			,	name TEXT NOT NULL
			,	UNIQUE(name)
			);

			CREATE TABLE albums
			(	id INTEGER PRIMARY KEY NOT NULL	
			,	title TEXT NOT NULL
			,	year INTEGER
			,	artwork TEXT
			,	artist INTEGER NOT NULL
			,	FOREIGN KEY(artist) REFERENCES artists(id)
			,	UNIQUE(artist, title)
			);

			CREATE TABLE directories
			(	path TEXT PRIMARY KEY NOT NULL
			,	name TEXT NOT NULL
			,	album INTEGER
			,	artwork TEXT
			,	FOREIGN KEY(album) REFERENCES albums(id)	
			);

    		CREATE TABLE songs
			(	path TEXT PRIMARY KEY NOT NULL
			,	track_number INTEGER
			,	title TEXT
			,	artist INTEGER
			,	album INTEGER
			,	FOREIGN KEY(artist) REFERENCES artists(id)
			,	FOREIGN KEY(album) REFERENCES albums(id)
			);

		").unwrap();

		Ok(index)
	}

	fn connect(&self) -> sqlite::Connection {
		sqlite::open(self.path.clone()).unwrap()
	}

	fn refresh(&self, vfs: &Vfs) {
		let db = self.connect();
		let mount_points = vfs.get_mount_points();
		for (_, target) in mount_points {
			self.populate_directory(&db, target.as_path());
		}
	}

	fn populate_directory(&self, db: &sqlite::Connection, path: &Path) {
		
	}

}

pub fn run(vfs: Arc<Vfs>, index: Arc<Index>) {
	loop {
		index.deref().refresh(vfs.deref());
		thread::sleep(time::Duration::from_secs(60 * 30)); // TODO expose in configuration
	}
}
