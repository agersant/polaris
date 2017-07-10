use core::ops::Deref;
use diesel;
use diesel::prelude::*;
use diesel::BelongingToDsl;

use db::{self, ConnectionSource};
use db::{playlists, users};
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
struct Playlist {
	id: i32,
	owner: i32,
}

struct PlaylistSong {
}

fn list_playlists<T>(owner: &str, db: &T) -> Result<Vec<String>>
	where T: ConnectionSource + VFSSource
{
	let connection = db.get_connection();
	let connection = connection.lock().unwrap();
	let connection = connection.deref();

	let user : User;
	{
	 	use self::users::dsl::*;
		user = users.filter(name.eq(owner)).select((id, name)).first(connection)?;
	}
	
	{
		use self::playlists::dsl::*;
		let found_playlists : Vec<String> = Playlist::belonging_to(&user).select(name).load(connection)?;
		Ok(found_playlists)
	}
}

fn save_playlist<T>(name: &str, owner: &str, content: &Vec<PlaylistSong>, db: &T) -> Result<()>
	where T: ConnectionSource + VFSSource
{
	let connection = db.get_connection();
	let connection = connection.lock().unwrap();
	let connection = connection.deref();

	let new_playlist = NewPlaylist {
		name: name.into(),
		owner: users::table
			.filter(users::columns::name.eq(owner))
			.select(users::columns::id)
			.get_result(connection)?,
	};

	diesel::insert(&new_playlist)
		.into(playlists::table)
		.execute(connection)?;

	Ok(())
}

fn delete_playlist<T>(playlist_name: &str, owner: &str, db: &T) -> Result<()>
	where T: ConnectionSource + VFSSource
{
	let connection = db.get_connection();
	let connection = connection.lock().unwrap();
	let connection = connection.deref();

	let user : User;
	{
	 	use self::users::dsl::*;
		user = users.filter(name.eq(owner)).select((id, name)).first(connection)?;
	}
	
	{
		use self::playlists::dsl::*;
		let q = Playlist::belonging_to(&user).filter(name.eq(playlist_name));
		diesel::delete(q).execute(connection)?;
	}
	Ok(())
}

#[test]
fn test_create_playlist() {
	let db = db::_get_test_db("create_playlist.sqlite");
	let playlist_content = Vec::new();

	let found_playlists = list_playlists("test_user", &db).unwrap();
	assert!(found_playlists.is_empty());

	save_playlist("chill_and_grill", "test_user", &playlist_content, &db).unwrap();
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
