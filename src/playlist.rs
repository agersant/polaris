use core::clone::Clone;
use core::ops::Deref;
use diesel;
use diesel::expression::sql;
use diesel::prelude::*;
use diesel::BelongingToDsl;
use diesel::types::*;
use std::path::Path;
#[cfg(test)]
use db;
use db::ConnectionSource;
use db::{playlists, playlist_songs, users};
use index::Song;
use vfs::VFSSource;
use errors::*;

#[derive(Insertable)]
#[table_name="playlists"]
struct NewPlaylist {
	name: String,
	owner: i32,
}

#[derive(Identifiable, Queryable)]
pub struct User {
	id: i32,
	name: String,
}

#[derive(Identifiable, Queryable, Associations)]
#[belongs_to(User, foreign_key="owner")]
pub struct Playlist {
	id: i32,
	owner: i32,
	name: String,
}

#[derive(Identifiable, Queryable, Associations)]
#[belongs_to(Playlist, foreign_key="playlist")]
pub struct PlaylistSong {
	id: i32,
	playlist: i32,
	path: String,
	ordering: i32,
}

#[derive(Insertable)]
#[table_name="playlist_songs"]
pub struct NewPlaylistSong {
	playlist: i32,
	path: String,
	ordering: i32,
}

fn list_playlists<T>(owner: &str, db: &T) -> Result<Vec<String>>
	where T: ConnectionSource + VFSSource
{
	let connection = db.get_connection();

	let user: User;
	{
		use self::users::dsl::*;
		user = users
			.filter(name.eq(owner))
			.select((id, name))
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

fn save_playlist<T>(name: &str, owner: &str, content: &Vec<String>, db: &T) -> Result<()>
	where T: ConnectionSource + VFSSource
{
	// TODO transaction for content delete+add
	let user: User;
	let new_playlist: NewPlaylist;
	let playlist: Playlist;

	{
		let connection = db.get_connection();

		// Find owner
		{
			use self::users::dsl::*;
			user = users
				.filter(name.eq(owner))
				.select((id, name))
				.get_result(connection.deref())?;
		}

		// Create playlist
		new_playlist = NewPlaylist {
			name: name.into(),
			owner: user.id,
		};

		diesel::insert(&new_playlist)
			.into(playlists::table)
			.execute(connection.deref())?;

		{
			use self::playlists::dsl::*;
			playlist = playlists
				.filter(name.eq(name).and(owner.eq(user.id)))
				.get_result(connection.deref())?;
		}

		// Delete old content (if any)
		let old_songs = PlaylistSong::belonging_to(&playlist);
		diesel::delete(old_songs).execute(connection.deref())?;
	}

	// Insert content
	let vfs = db.get_vfs()?;
	let mut new_songs: Vec<NewPlaylistSong> = Vec::new();
	new_songs.reserve(content.len());
	for (i, path) in content.iter().enumerate() {
		let virtual_path = Path::new(&path);
		if let Some(real_path) = vfs.virtual_to_real(virtual_path)
		       .ok()
		       .and_then(|p| p.to_str().map(|s| s.to_owned())) {
			new_songs.push(NewPlaylistSong {
			                   playlist: playlist.id,
			                   path: real_path.into(),
			                   ordering: i as i32,
			               });
		}
	}

	{
		let connection = db.get_connection();
		diesel::insert(&new_songs)
			.into(playlist_songs::table)
			.execute(connection.deref())?;
	}

	Ok(())
}

fn read_playlist<T>(playlist_name: &str, owner: &str, db: &T) -> Result<Vec<Song>>
	where T: ConnectionSource + VFSSource
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
				.select((id, name))
				.get_result(connection.deref())?;
		}

		// Find playlist
		{
			use self::playlists::dsl::*;
			playlist = playlists
				.filter(name.eq(playlist_name).and(owner.eq(user.id)))
				.get_result(connection.deref())?;
		}

		// Select songs. Not using Diesel because we need to LEFT JOIN using a custom column
		let query = sql::<(Integer, Text, Text, Nullable<Integer>, Nullable<Integer>, Nullable<Text>, Nullable<Text>, Nullable<Text>, Nullable<Integer>, Nullable<Text>, Nullable<Text>)>(r#"
			SELECT s.id, s.path, s.parent, s.track_number, s.disc_number, s.title, s.artist, s.album_artist, s.year, s.album, s.artwork
			FROM playlist_songs ps
			LEFT JOIN songs s ON ps.path = s.path
			WHERE ps.playlist = ?
			ORDER BY ps.ordering
		"#);
		let query = query.clone().bind::<Integer, _>(playlist.id);
		songs = query.get_results(connection.deref())?;
	}

	// Map real path to virtual paths
	let songs = songs.into_iter().filter_map(|mut s| {
		let real_path = s.path.clone();
		let real_path = Path::new(&real_path);
		if let Ok(virtual_path) = vfs.real_to_virtual(real_path) {
			if let Some(virtual_path) = virtual_path.to_str() {
				s.path = virtual_path.to_owned();
			}
			return Some(s);
		}
		None
	}).collect();

	Ok(songs)
}

fn delete_playlist<T>(playlist_name: &str, owner: &str, db: &T) -> Result<()>
	where T: ConnectionSource + VFSSource
{
	let connection = db.get_connection();

	let user: User;
	{
		use self::users::dsl::*;
		user = users
			.filter(name.eq(owner))
			.select((id, name))
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
	use index;

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
}
