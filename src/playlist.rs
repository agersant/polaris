use anyhow::*;
use core::clone::Clone;
use diesel;
use diesel::prelude::*;
use diesel::sql_types;
use diesel::BelongingToDsl;
use std::path::Path;
use thiserror::Error;

#[cfg(test)]
use crate::db;
use crate::db::DB;
use crate::db::{playlist_songs, playlists, users};
use crate::index::{self, Song};
use crate::vfs::VFSSource;

#[derive(Error, Debug)]
pub enum PlaylistError {
	#[error("User not found")]
	UserNotFound,
	#[error("Playlist not found")]
	PlaylistNotFound,
	#[error("Unspecified")]
	Unspecified,
}

impl From<anyhow::Error> for PlaylistError {
	fn from(_: anyhow::Error) -> Self {
		PlaylistError::Unspecified
	}
}

#[derive(Insertable)]
#[table_name = "playlists"]
struct NewPlaylist {
	name: String,
	owner: i32,
}

#[derive(Identifiable, Queryable)]
pub struct User {
	id: i32,
}

#[derive(Identifiable, Queryable, Associations)]
#[belongs_to(User, foreign_key = "owner")]
pub struct Playlist {
	id: i32,
	owner: i32,
}

#[derive(Identifiable, Queryable, Associations)]
#[belongs_to(Playlist, foreign_key = "playlist")]
pub struct PlaylistSong {
	id: i32,
	playlist: i32,
}

#[derive(Insertable)]
#[table_name = "playlist_songs"]
pub struct NewPlaylistSong {
	playlist: i32,
	path: String,
	ordering: i32,
}

pub fn list_playlists(owner: &str, db: &DB) -> Result<Vec<String>, PlaylistError> {
	let connection = db.connect()?;

	let user: User = {
		use self::users::dsl::*;
		users
			.filter(name.eq(owner))
			.select((id,))
			.first(&connection)
			.optional()
			.map_err(anyhow::Error::new)?
			.ok_or(PlaylistError::UserNotFound)?
	};

	{
		use self::playlists::dsl::*;
		let found_playlists: Vec<String> = Playlist::belonging_to(&user)
			.select(name)
			.load(&connection)
			.map_err(anyhow::Error::new)?;
		Ok(found_playlists)
	}
}

pub fn save_playlist(
	playlist_name: &str,
	owner: &str,
	content: &[String],
	db: &DB,
) -> Result<(), PlaylistError> {
	let new_playlist: NewPlaylist;
	let playlist: Playlist;
	let vfs = db.get_vfs()?;

	{
		let connection = db.connect()?;

		// Find owner
		let user: User = {
			use self::users::dsl::*;
			users
				.filter(name.eq(owner))
				.select((id,))
				.first(&connection)
				.optional()
				.map_err(anyhow::Error::new)?
				.ok_or(PlaylistError::UserNotFound)?
		};

		// Create playlist
		new_playlist = NewPlaylist {
			name: playlist_name.into(),
			owner: user.id,
		};

		diesel::insert_into(playlists::table)
			.values(&new_playlist)
			.execute(&connection)
			.map_err(anyhow::Error::new)?;

		playlist = {
			use self::playlists::dsl::*;
			playlists
				.select((id, owner))
				.filter(name.eq(playlist_name).and(owner.eq(user.id)))
				.get_result(&connection)
				.map_err(anyhow::Error::new)?
		}
	}

	let mut new_songs: Vec<NewPlaylistSong> = Vec::new();
	new_songs.reserve(content.len());

	for (i, path) in content.iter().enumerate() {
		let virtual_path = Path::new(&path);
		if let Some(real_path) = vfs
			.virtual_to_real(virtual_path)
			.ok()
			.and_then(|p| p.to_str().map(|s| s.to_owned()))
		{
			new_songs.push(NewPlaylistSong {
				playlist: playlist.id,
				path: real_path,
				ordering: i as i32,
			});
		}
	}

	{
		let connection = db.connect()?;
		connection
			.transaction::<_, diesel::result::Error, _>(|| {
				// Delete old content (if any)
				let old_songs = PlaylistSong::belonging_to(&playlist);
				diesel::delete(old_songs).execute(&connection)?;

				// Insert content
				diesel::insert_into(playlist_songs::table)
					.values(&new_songs)
					.execute(&*connection)?; // TODO https://github.com/diesel-rs/diesel/issues/1822
				Ok(())
			})
			.map_err(anyhow::Error::new)?;
	}

	Ok(())
}

pub fn read_playlist(
	playlist_name: &str,
	owner: &str,
	db: &DB,
) -> Result<Vec<Song>, PlaylistError> {
	let vfs = db.get_vfs()?;
	let songs: Vec<Song>;

	{
		let connection = db.connect()?;

		// Find owner
		let user: User = {
			use self::users::dsl::*;
			users
				.filter(name.eq(owner))
				.select((id,))
				.first(&connection)
				.optional()
				.map_err(anyhow::Error::new)?
				.ok_or(PlaylistError::UserNotFound)?
		};

		// Find playlist
		let playlist: Playlist = {
			use self::playlists::dsl::*;
			playlists
				.select((id, owner))
				.filter(name.eq(playlist_name).and(owner.eq(user.id)))
				.get_result(&connection)
				.optional()
				.map_err(anyhow::Error::new)?
				.ok_or(PlaylistError::PlaylistNotFound)?
		};

		// Select songs. Not using Diesel because we need to LEFT JOIN using a custom column
		let query = diesel::sql_query(
			r#"
			SELECT s.id, s.path, s.parent, s.track_number, s.disc_number, s.title, s.artist, s.album_artist, s.year, s.album, s.artwork, s.duration
			FROM playlist_songs ps
			LEFT JOIN songs s ON ps.path = s.path
			WHERE ps.playlist = ?
			ORDER BY ps.ordering
		"#,
		);
		let query = query.clone().bind::<sql_types::Integer, _>(playlist.id);
		songs = query.get_results(&connection).map_err(anyhow::Error::new)?;
	}

	// Map real path to virtual paths
	let virtual_songs = songs
		.into_iter()
		.filter_map(|s| index::virtualize_song(&vfs, s))
		.collect();

	Ok(virtual_songs)
}

pub fn delete_playlist(playlist_name: &str, owner: &str, db: &DB) -> Result<(), PlaylistError> {
	let connection = db.connect()?;

	let user: User = {
		use self::users::dsl::*;
		users
			.filter(name.eq(owner))
			.select((id,))
			.first(&connection)
			.optional()
			.map_err(anyhow::Error::new)?
			.ok_or(PlaylistError::UserNotFound)?
	};

	{
		use self::playlists::dsl::*;
		let q = Playlist::belonging_to(&user).filter(name.eq(playlist_name));
		match diesel::delete(q)
			.execute(&connection)
			.map_err(anyhow::Error::new)?
		{
			0 => Err(PlaylistError::PlaylistNotFound),
			_ => Ok(()),
		}
	}
}

#[test]
fn test_create_playlist() {
	let db = db::get_test_db("create_playlist.sqlite");

	let found_playlists = list_playlists("test_user", &db).unwrap();
	assert!(found_playlists.is_empty());

	save_playlist("chill_and_grill", "test_user", &Vec::new(), &db).unwrap();
	let found_playlists = list_playlists("test_user", &db).unwrap();
	assert_eq!(found_playlists.len(), 1);
	assert_eq!(found_playlists[0], "chill_and_grill");

	let found_playlists = list_playlists("someone_else", &db);
	assert!(found_playlists.is_err());
}

#[test]
fn test_delete_playlist() {
	let db = db::get_test_db("delete_playlist.sqlite");
	let playlist_content = Vec::new();

	save_playlist("chill_and_grill", "test_user", &playlist_content, &db).unwrap();
	save_playlist("mellow_bungalow", "test_user", &playlist_content, &db).unwrap();
	let found_playlists = list_playlists("test_user", &db).unwrap();
	assert_eq!(found_playlists.len(), 2);

	delete_playlist("chill_and_grill", "test_user", &db).unwrap();
	let found_playlists = list_playlists("test_user", &db).unwrap();
	assert_eq!(found_playlists.len(), 1);
	assert_eq!(found_playlists[0], "mellow_bungalow");

	let delete_result = delete_playlist("mellow_bungalow", "someone_else", &db);
	assert!(delete_result.is_err());
}

#[test]
fn test_fill_playlist() {
	use crate::index;

	let db = db::get_test_db("fill_playlist.sqlite");
	index::update(&db).unwrap();

	let mut playlist_content: Vec<String> = index::flatten(&db, Path::new("root"))
		.unwrap()
		.into_iter()
		.map(|s| s.path)
		.collect();
	assert_eq!(playlist_content.len(), 13);

	let first_song = playlist_content[0].clone();
	playlist_content.push(first_song);
	assert_eq!(playlist_content.len(), 14);

	save_playlist("all_the_music", "test_user", &playlist_content, &db).unwrap();

	let songs = read_playlist("all_the_music", "test_user", &db).unwrap();
	assert_eq!(songs.len(), 14);
	assert_eq!(songs[0].title, Some("Above The Water".to_owned()));
	assert_eq!(songs[13].title, Some("Above The Water".to_owned()));

	use std::path::PathBuf;
	let mut first_song_path = PathBuf::new();
	first_song_path.push("root");
	first_song_path.push("Khemmis");
	first_song_path.push("Hunted");
	first_song_path.push("01 - Above The Water.mp3");
	assert_eq!(songs[0].path, first_song_path.to_str().unwrap());

	// Save again to verify that we don't dupe the content
	save_playlist("all_the_music", "test_user", &playlist_content, &db).unwrap();
	let songs = read_playlist("all_the_music", "test_user", &db).unwrap();
	assert_eq!(songs.len(), 14);
}
