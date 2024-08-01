use rustfm_scrobble::{Scrobble, Scrobbler};
use std::path::Path;
use user::AuthToken;

use crate::app::{index, user, Error};

const LASTFM_API_KEY: &str = "02b96c939a2b451c31dfd67add1f696e";
const LASTFM_API_SECRET: &str = "0f25a80ceef4b470b5cb97d99d4b3420";

#[derive(Clone)]
pub struct Manager {
	index_manager: index::Manager,
	user_manager: user::Manager,
}

impl Manager {
	pub fn new(index_manager: index::Manager, user_manager: user::Manager) -> Self {
		Self {
			index_manager,
			user_manager,
		}
	}

	pub fn generate_link_token(&self, username: &str) -> Result<AuthToken, Error> {
		self.user_manager
			.generate_lastfm_link_token(username)
			.map_err(|e| e.into())
	}

	pub async fn link(&self, username: &str, lastfm_token: &str) -> Result<(), Error> {
		let mut scrobbler = Scrobbler::new(LASTFM_API_KEY, LASTFM_API_SECRET);
		let auth_response = scrobbler
			.authenticate_with_token(lastfm_token)
			.map_err(Error::ScrobblerAuthentication)?;

		self.user_manager
			.lastfm_link(username, &auth_response.name, &auth_response.key)
			.await
			.map_err(|e| e.into())
	}

	pub async fn unlink(&self, username: &str) -> Result<(), Error> {
		self.user_manager
			.lastfm_unlink(username)
			.await
			.map_err(|e| e.into())
	}

	pub async fn scrobble(&self, username: &str, track: &Path) -> Result<(), Error> {
		let mut scrobbler = Scrobbler::new(LASTFM_API_KEY, LASTFM_API_SECRET);
		let scrobble = self.scrobble_from_path(track).await?;
		let auth_token = self.user_manager.get_lastfm_session_key(username).await?;
		scrobbler.authenticate_with_session_key(&auth_token);
		scrobbler.scrobble(&scrobble).map_err(Error::Scrobble)?;
		Ok(())
	}

	pub async fn now_playing(&self, username: &str, track: &Path) -> Result<(), Error> {
		let mut scrobbler = Scrobbler::new(LASTFM_API_KEY, LASTFM_API_SECRET);
		let scrobble = self.scrobble_from_path(track).await?;
		let auth_token = self.user_manager.get_lastfm_session_key(username).await?;
		scrobbler.authenticate_with_session_key(&auth_token);
		scrobbler
			.now_playing(&scrobble)
			.map_err(Error::NowPlaying)?;
		Ok(())
	}

	async fn scrobble_from_path(&self, track: &Path) -> Result<Scrobble, Error> {
		let song_key = index::SongKey {
			virtual_path: track.to_owned(),
		};
		let song = self.index_manager.get_song(&song_key).await?;
		Ok(Scrobble::new(
			song.artists.first().map(|s| s.as_str()).unwrap_or(""),
			song.title.as_deref().unwrap_or(""),
			song.album.as_deref().unwrap_or(""),
		))
	}
}
