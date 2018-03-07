use rustfm_scrobble::{Scrobbler, Scrobble};
use std::path::Path;

use db::ConnectionSource;
use errors::*;
use index;
use user;
use vfs::VFSSource;

const LASTFM_API_KEY: &str = "02b96c939a2b451c31dfd67add1f696e";
const LASTFM_API_SECRET: &str = "0f25a80ceef4b470b5cb97d99d4b3420";

fn scrobble_from_path<T>(db: &T, track: &Path) -> Result<Scrobble>
	where T: ConnectionSource + VFSSource
{
	let song = index::get_song(db, track)?;
	Ok(Scrobble::new(song.artist.unwrap_or("".into()),
	                 song.title.unwrap_or("".into()),
	                 song.album.unwrap_or("".into())))
}

pub fn scrobble<T>(db: &T, username: &str, track: &Path) -> Result<()>
	where T: ConnectionSource + VFSSource
{
	let mut scrobbler = Scrobbler::new(LASTFM_API_KEY.into(), LASTFM_API_SECRET.into());
	let scrobble = scrobble_from_path(db, track)?;
	let (lastfm_username, lastfm_password) = user::get_lastfm_credentials(db, username)?;
	scrobbler
		.authenticate_with_password(lastfm_username, lastfm_password)?;
	scrobbler.scrobble(scrobble)?;
	Ok(())
}

pub fn now_playing<T>(db: &T, username: &str, track: &Path) -> Result<()>
	where T: ConnectionSource + VFSSource
{
	let mut scrobbler = Scrobbler::new(LASTFM_API_KEY.into(), LASTFM_API_SECRET.into());
	let scrobble = scrobble_from_path(db, track)?;
	let (lastfm_username, lastfm_password) = user::get_lastfm_credentials(db, username)?;
	scrobbler
		.authenticate_with_password(lastfm_username, lastfm_password)?;
	scrobbler.now_playing(scrobble)?;
	Ok(())
}
