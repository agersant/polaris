use anyhow::*;
use crossbeam_channel::{Receiver, Sender};
use diesel;
use diesel::prelude::*;
#[cfg(feature = "profile-index")]
use flame;
use log::{error, info};
use rayon::prelude::*;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::time;

use crate::config::MiscSettings;
use crate::db::{directories, misc_settings, songs, DB};
use crate::index::metadata;
use crate::vfs::VFSSource;
use metadata::SongTags;

const INDEX_BUILDING_INSERT_BUFFER_SIZE: usize = 1000; // Insertions in each transaction
const INDEX_BUILDING_CLEAN_BUFFER_SIZE: usize = 500; // Insertions in each transaction

pub fn update(db: &DB) -> Result<()> {
	let start = time::Instant::now();
	info!("Beginning library index update");
	clean(db)?;
	populate(db)?;
	info!(
		"Library index update took {} seconds",
		start.elapsed().as_millis() as f32 / 1000.0
	);
	#[cfg(feature = "profile-index")]
	flame::dump_html(&mut fs::File::create("index-flame-graph.html").unwrap()).unwrap();
	Ok(())
}

#[derive(Debug, Insertable)]
#[table_name = "songs"]
struct NewSong {
	path: String,
	parent: String,
	track_number: Option<i32>,
	disc_number: Option<i32>,
	title: Option<String>,
	artist: Option<String>,
	album_artist: Option<String>,
	year: Option<i32>,
	album: Option<String>,
	artwork: Option<String>,
	duration: Option<i32>,
}

#[derive(Debug, Insertable)]
#[table_name = "directories"]
struct NewDirectory {
	path: String,
	parent: Option<String>,
	artist: Option<String>,
	year: Option<i32>,
	album: Option<String>,
	artwork: Option<String>,
	date_added: i32,
}

struct IndexUpdater {
	directory_sender: Sender<NewDirectory>,
	song_sender: Sender<NewSong>,
	album_art_pattern: Regex,
}

impl IndexUpdater {
	#[cfg_attr(feature = "profile-index", flame)]
	fn new(
		album_art_pattern: Regex,
		directory_sender: Sender<NewDirectory>,
		song_sender: Sender<NewSong>,
	) -> Result<IndexUpdater> {
		Ok(IndexUpdater {
			directory_sender,
			song_sender,
			album_art_pattern,
		})
	}

	#[cfg_attr(feature = "profile-index", flame)]
	fn push_song(&self, song: NewSong) -> Result<()> {
		self.song_sender.send(song).map_err(Error::new)
	}

	#[cfg_attr(feature = "profile-index", flame)]
	fn push_directory(&self, directory: NewDirectory) -> Result<()> {
		self.directory_sender.send(directory).map_err(Error::new)
	}

	fn get_artwork(&self, dir: &Path) -> Result<Option<String>> {
		for file in fs::read_dir(dir)? {
			let file = file?;
			if let Some(name_string) = file.file_name().to_str() {
				if self.album_art_pattern.is_match(name_string) {
					return Ok(file.path().to_str().map(|p| p.to_owned()));
				}
			}
		}
		Ok(None)
	}

	fn populate_directory(&self, parent: Option<&Path>, path: &Path) -> Result<()> {
		#[cfg(feature = "profile-index")]
		let _guard = flame::start_guard(format!(
			"dir: {}",
			path.file_name()
				.map(|s| { s.to_string_lossy().into_owned() })
				.unwrap_or("Unknown".to_owned())
		));

		// Find artwork
		let mut directory_artwork = {
			#[cfg(feature = "profile-index")]
			let _guard = flame::start_guard("artwork");
			self.get_artwork(path).unwrap_or(None)
		};

		// Extract path and parent path
		let parent_string = parent.and_then(|p| p.to_str()).map(|s| s.to_owned());
		let path_string = path.to_str().ok_or(anyhow!("Invalid directory path"))?;

		// Find date added
		let metadata = {
			#[cfg(feature = "profile-index")]
			let _guard = flame::start_guard("metadata");
			fs::metadata(path_string)?
		};
		let created = {
			#[cfg(feature = "profile-index")]
			let _guard = flame::start_guard("created_date");
			metadata
				.created()
				.or_else(|_| metadata.modified())?
				.duration_since(time::UNIX_EPOCH)?
				.as_secs() as i32
		};

		let mut directory_album = None;
		let mut directory_year = None;
		let mut directory_artist = None;
		let mut inconsistent_directory_album = false;
		let mut inconsistent_directory_year = false;
		let mut inconsistent_directory_artist = false;

		// Sub directories
		let mut sub_directories = Vec::new();
		let mut song_files = Vec::new();

		let files = match fs::read_dir(path) {
			Ok(files) => files,
			Err(e) => {
				error!("Directory read error for `{}`: {}", path.display(), e);
				return Err(e.into());
			}
		};

		// Insert content
		for file in files {
			let file_path = match file {
				Ok(ref f) => f.path(),
				Err(e) => {
					error!("File read error within `{}`: {}", path_string, e);
					break;
				}
			};

			#[cfg(feature = "profile-index")]
			let _guard = flame::start_guard(format!(
				"file: {}",
				file_path
					.as_path()
					.file_name()
					.map(|s| { s.to_string_lossy().into_owned() })
					.unwrap_or("Unknown".to_owned())
			));

			if file_path.is_dir() {
				sub_directories.push(file_path.to_path_buf());
				continue;
			}

			song_files.push(file_path);
		}

		let song_metadata = |path: PathBuf| -> Option<(String, SongTags)> {
			#[cfg(feature = "profile-index")]
			let _guard = flame::start_guard("song_metadata");

			path.to_str().and_then(|file_path_string| {
				metadata::read(&path).map(|m| (file_path_string.to_owned(), m))
			})
		};
		let song_tags = song_files
			.into_par_iter()
			.filter_map(song_metadata)
			.collect::<Vec<_>>();

		if directory_artwork.is_none() {
			directory_artwork = song_tags
				.iter()
				.find(|(_, t)| t.has_artwork)
				.map(|(p, _)| p.to_owned());
		}

		for (file_path_string, tags) in song_tags {
			if tags.year.is_some() {
				inconsistent_directory_year |=
					directory_year.is_some() && directory_year != tags.year;
				directory_year = tags.year;
			}

			if tags.album.is_some() {
				inconsistent_directory_album |=
					directory_album.is_some() && directory_album != tags.album;
				directory_album = tags.album.as_ref().cloned();
			}

			if tags.album_artist.is_some() {
				inconsistent_directory_artist |=
					directory_artist.is_some() && directory_artist != tags.album_artist;
				directory_artist = tags.album_artist.as_ref().cloned();
			} else if tags.artist.is_some() {
				inconsistent_directory_artist |=
					directory_artist.is_some() && directory_artist != tags.artist;
				directory_artist = tags.artist.as_ref().cloned();
			}

			let artwork_path = if tags.has_artwork {
				Some(file_path_string.to_owned())
			} else {
				directory_artwork.as_ref().cloned()
			};

			let song = NewSong {
				path: file_path_string.to_owned(),
				parent: path_string.to_owned(),
				disc_number: tags.disc_number.map(|n| n as i32),
				track_number: tags.track_number.map(|n| n as i32),
				title: tags.title,
				duration: tags.duration.map(|n| n as i32),
				artist: tags.artist,
				album_artist: tags.album_artist,
				album: tags.album,
				year: tags.year,
				artwork: artwork_path,
			};

			self.push_song(song)?;
		}

		// Insert directory
		let directory = {
			if inconsistent_directory_year {
				directory_year = None;
			}
			if inconsistent_directory_album {
				directory_album = None;
			}
			if inconsistent_directory_artist {
				directory_artist = None;
			}

			NewDirectory {
				path: path_string.to_owned(),
				parent: parent_string,
				artwork: directory_artwork,
				album: directory_album,
				artist: directory_artist,
				year: directory_year,
				date_added: created,
			}
		};

		self.push_directory(directory)?;

		// Populate subdirectories
		sub_directories
			.into_par_iter()
			.map(|sub_directory| self.populate_directory(Some(path), &sub_directory))
			.collect() // propagate an error to the caller if one of them failed
	}
}

#[cfg_attr(feature = "profile-index", flame)]
pub fn clean(db: &DB) -> Result<()> {
	let vfs = db.get_vfs()?;

	{
		let all_songs: Vec<String>;
		{
			let connection = db.connect()?;
			all_songs = songs::table.select(songs::path).load(&connection)?;
		}

		let missing_songs = all_songs
			.par_iter()
			.filter(|ref song_path| {
				let path = Path::new(&song_path);
				!path.exists() || vfs.real_to_virtual(path).is_err()
			})
			.collect::<Vec<_>>();

		{
			let connection = db.connect()?;
			for chunk in missing_songs[..].chunks(INDEX_BUILDING_CLEAN_BUFFER_SIZE) {
				diesel::delete(songs::table.filter(songs::path.eq_any(chunk)))
					.execute(&connection)?;
			}
		}
	}

	{
		let all_directories: Vec<String>;
		{
			let connection = db.connect()?;
			all_directories = directories::table
				.select(directories::path)
				.load(&connection)?;
		}

		let missing_directories = all_directories
			.par_iter()
			.filter(|ref directory_path| {
				let path = Path::new(&directory_path);
				!path.exists() || vfs.real_to_virtual(path).is_err()
			})
			.collect::<Vec<_>>();

		{
			let connection = db.connect()?;
			for chunk in missing_directories[..].chunks(INDEX_BUILDING_CLEAN_BUFFER_SIZE) {
				diesel::delete(directories::table.filter(directories::path.eq_any(chunk)))
					.execute(&connection)?;
			}
		}
	}

	Ok(())
}

#[cfg_attr(feature = "profile-index", flame)]
pub fn populate(db: &DB) -> Result<()> {
	let vfs = db.get_vfs()?;
	let mount_points = vfs.get_mount_points();

	let album_art_pattern = {
		let connection = db.connect()?;
		let settings: MiscSettings = misc_settings::table.get_result(&connection)?;
		Regex::new(&settings.index_album_art_pattern)?
	};

	let (directory_sender, directory_receiver) = crossbeam_channel::unbounded();
	let (song_sender, song_receiver) = crossbeam_channel::unbounded();

	let songs_db = db.clone();
	let directories_db = db.clone();

	let directories_thread = std::thread::spawn(move || {
		insert_directories(directory_receiver, directories_db);
	});

	let songs_thread = std::thread::spawn(move || {
		insert_songs(song_receiver, songs_db);
	});

	{
		let updater = IndexUpdater::new(album_art_pattern, directory_sender, song_sender)?;
		let mount_points = mount_points.values().collect::<Vec<_>>();
		mount_points
			.iter()
			.par_bridge()
			.map(|target| updater.populate_directory(None, target.as_path()))
			.collect::<Result<()>>()?;
	}

	match directories_thread.join() {
		Err(e) => error!(
			"Error while waiting for directory insertions to complete: {:?}",
			e
		),
		_ => (),
	}

	match songs_thread.join() {
		Err(e) => error!(
			"Error while waiting for song insertions to complete: {:?}",
			e
		),
		_ => (),
	}

	Ok(())
}

fn flush_directories(db: &DB, entries: &Vec<NewDirectory>) {
	if db
		.connect()
		.and_then(|connection| {
			diesel::insert_into(directories::table)
				.values(entries)
				.execute(&*connection) // TODO https://github.com/diesel-rs/diesel/issues/1822
				.map_err(Error::new)
		})
		.is_err()
	{
		error!("Could not insert new directories in database");
	}
}

fn flush_songs(db: &DB, entries: &Vec<NewSong>) {
	if db
		.connect()
		.and_then(|connection| {
			diesel::insert_into(songs::table)
				.values(entries)
				.execute(&*connection) // TODO https://github.com/diesel-rs/diesel/issues/1822
				.map_err(Error::new)
		})
		.is_err()
	{
		error!("Could not insert new songs in database");
	}
}

fn insert_directories(receiver: Receiver<NewDirectory>, db: DB) {
	let mut new_entries = Vec::new();
	new_entries.reserve_exact(INDEX_BUILDING_INSERT_BUFFER_SIZE);

	loop {
		match receiver.recv() {
			Ok(s) => {
				new_entries.push(s);
				if new_entries.len() >= INDEX_BUILDING_INSERT_BUFFER_SIZE {
					flush_directories(&db, &new_entries);
					new_entries.clear();
				}
			}
			Err(_) => break,
		}
	}

	if new_entries.len() > 0 {
		flush_directories(&db, &new_entries);
	}
}

fn insert_songs(receiver: Receiver<NewSong>, db: DB) {
	let mut new_entries = Vec::new();
	new_entries.reserve_exact(INDEX_BUILDING_INSERT_BUFFER_SIZE);

	loop {
		match receiver.recv() {
			Ok(s) => {
				new_entries.push(s);
				if new_entries.len() >= INDEX_BUILDING_INSERT_BUFFER_SIZE {
					flush_songs(&db, &new_entries);
					new_entries.clear();
				}
			}
			Err(_) => break,
		}
	}

	if new_entries.len() > 0 {
		flush_songs(&db, &new_entries);
	}
}
