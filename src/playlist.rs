use core::clone::Clone;
use core::ops::Deref;
use diesel;
use diesel::prelude::*;
use diesel::sql_types;
use diesel::BelongingToDsl;
use std::path::Path;

#[cfg(test)]
use crate::db;
use crate::db::ConnectionSource;
use crate::db::{playlist_songs, playlists, users};
use crate::errors::*;
use crate::index::{self, Song};
use crate::vfs::VFSSource;

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

pub fn list_playlists<T>(owner: &str, db: &T) -> Result<Vec<String>>
where
	T: ConnectionSource + VFSSource,
{
	let connection = db.get_connection();

	let user: User;
	{
		use self::users::dsl::*;
		user = users
			.filter(name.eq(owner))
			.select((id,))
			.first(connection.deref())?;
	}

	{
		use self::playlists::dsl::*;
		let found_playlists: Vec<String> = Playlist::belonging_to(&user)
			.select(name)
			.load(connection.deref())?;
		Ok(found_playlists)
	}
}

pub fn save_playlist<T>(playlist_name: &str, owner: &str, content: &[String], db: &T) -> Result<()>
where
	T: ConnectionSource + VFSSource,
{
	let user: User;
	let new_playlist: NewPlaylist;
	let playlist: Playlist;
	let vfs = db.get_vfs()?;

	{
		let connection = db.get_connection();

		// Find owner
		{
			use self::users::dsl::*;
			user = users
				.filter(name.eq(owner))
				.select((id,))
				.get_result(connection.deref())?;
		}

		// Create playlist
		new_playlist = NewPlaylist {
			name: playlist_name.into(),
			owner: user.id,
		};

		diesel::insert_into(playlists::table)
			.values(&new_playlist)
			.execute(connection.deref())?;

		{
			use self::playlists::dsl::*;
			playlist = playlists
				.select((id, owner))
				.filter(name.eq(playlist_name).and(owner.eq(user.id)))
				.get_result(connection.deref())?;
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
		let connection = db.get_connection();
		connection
			.deref()
			.transaction::<_, diesel::result::Error, _>(|| {
				// Delete old content (if any)
				let old_songs = PlaylistSong::belonging_to(&playlist);
				diesel::delete(old_songs).execute(connection.deref())?;

				// Insert content
				diesel::insert_into(playlist_songs::table)
					.values(&new_songs)
					.execute(connection.deref())?;
				Ok(())
			})?;
	}

	Ok(())
}

pub fn read_playlist<T>(playlist_name: &str, owner: &str, db: &T) -> Result<Vec<Song>>
where
	T: ConnectionSource + VFSSource,
{
	let vfs = db.get_vfs()?;
	let songs: Vec<Song>;

	{
		let connection = db.get_connection();
		let user: User;
		let playlist: Playlist;

		// Find owner
		{
			use self::users::dsl::*;
			user = users
				.filter(name.eq(owner))
				.select((id,))
				.get_result(connection.deref())?;
		}

		// Find playlist
		{
			use self::playlists::dsl::*;
			playlist = playlists
				.select((id, owner))
				.filter(name.eq(playlist_name).and(owner.eq(user.id)))
				.get_result(connection.deref())?;
		}

		// Select songs. Not using Diesel because we need to LEFT JOIN using a custom column
		let query = diesel::sql_query(r#"
			SELECT s.id, s.path, s.parent, s.track_number, s.disc_number, s.title, s.artist, s.album_artist, s.year, s.album, s.artwork, s.duration
			FROM playlist_songs ps
			LEFT JOIN songs s ON ps.path = s.path
			WHERE ps.playlist = ?
			ORDER BY ps.ordering
		"#);
		let query = query.clone().bind::<sql_types::Integer, _>(playlist.id);
		songs = query.get_results(connection.deref())?;
	}

	// Map real path to virtual paths
	let virtual_songs = songs
		.into_iter()
		.filter_map(|s| index::virtualize_song(&vfs, s))
		.collect();

	Ok(virtual_songs)
}

pub fn delete_playlist<T>(playlist_name: &str, owner: &str, db: &T) -> Result<()>
where
	T: ConnectionSource + VFSSource,
{
	let connection = db.get_connection();

	let user: User;
	{
		use self::users::dsl::*;
		user = users
			.filter(name.eq(owner))
			.select((id,))
			.first(connection.deref())?;
	}

	{
		use self::playlists::dsl::*;
		let q = Playlist::belonging_to(&user).filter(name.eq(playlist_name));
		diesel::delete(q).execute(connection.deref())?;
	}

	Ok(())
}

#[test]
fn test_create_playlist() {
	let db = db::_get_test_db("create_playlist.sqlite");

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
	let db = db::_get_test_db("delete_playlist.sqlite");
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

	let db = db::_get_test_db("fill_playlist.sqlite");
	index::update(&db).unwrap();

	let mut playlist_content: Vec<String> = index::flatten(&db, Path::new("root"))
		.unwrap()
		.into_iter()
		.map(|s| s.path)
		.collect();
	assert_eq!(playlist_content.len(), 12);

	let first_song = playlist_content[0].clone();
	playlist_content.push(first_song);
	assert_eq!(playlist_content.len(), 13);

	save_playlist("all_the_music", "test_user", &playlist_content, &db).unwrap();

	let songs = read_playlist("all_the_music", "test_user", &db).unwrap();
	assert_eq!(songs.len(), 13);
	assert_eq!(songs[0].title, Some("Above The Water".to_owned()));
	assert_eq!(songs[12].title, Some("Above The Water".to_owned()));

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
	assert_eq!(songs.len(), 13);
}
