use diesel;
use diesel::prelude::*;
use regex::Regex;
use std::time::Duration;

use super::*;
use crate::app::user;
use crate::db::{ddns_config, misc_settings, mount_points, users, DB};

#[derive(Clone)]
pub struct Manager {
	pub db: DB,
	user_manager: user::Manager,
}

impl Manager {
	pub fn new(db: DB, user_manager: user::Manager) -> Self {
		Self { db, user_manager }
	}

	pub fn get_auth_secret(&self) -> Result<Vec<u8>, Error> {
		use self::misc_settings::dsl::*;
		let connection = self.db.connect()?;
		misc_settings
			.select(auth_secret)
			.get_result(&connection)
			.map_err(|e| match e {
				diesel::result::Error::NotFound => Error::AuthSecretNotFound,
				_ => Error::Unspecified,
			})
	}

	pub fn get_index_sleep_duration(&self) -> Result<Duration, Error> {
		use self::misc_settings::dsl::*;
		let connection = self.db.connect()?;
		misc_settings
			.select(index_sleep_duration_seconds)
			.get_result(&connection)
			.map_err(|e| match e {
				diesel::result::Error::NotFound => Error::IndexSleepDurationNotFound,
				_ => Error::Unspecified,
			})
			.map(|s: i32| Duration::from_secs(s as u64))
	}

	pub fn get_index_album_art_pattern(&self) -> Result<Regex, Error> {
		use self::misc_settings::dsl::*;
		let connection = self.db.connect()?;
		misc_settings
			.select(index_album_art_pattern)
			.get_result(&connection)
			.map_err(|e| match e {
				diesel::result::Error::NotFound => Error::IndexAlbumArtPatternNotFound,
				_ => Error::Unspecified,
			})
			.map(|s: String| format!("(?i){}", s))
			.and_then(|s| Regex::new(&s).map_err(|_| Error::IndexAlbumArtPatternInvalid))
	}

	pub fn read(&self) -> anyhow::Result<Config> {
		use self::ddns_config::dsl::*;
		use self::misc_settings::dsl::*;

		let connection = self.db.connect()?;

		let mut config = Config {
			album_art_pattern: None,
			reindex_every_n_seconds: None,
			mount_dirs: None,
			users: None,
			ydns: None,
		};

		let (art_pattern, sleep_duration) = misc_settings
			.select((index_album_art_pattern, index_sleep_duration_seconds))
			.get_result(&connection)?;

		config.album_art_pattern = Some(art_pattern);
		config.reindex_every_n_seconds = Some(sleep_duration);

		let mount_dirs;
		{
			use self::mount_points::dsl::*;
			mount_dirs = mount_points
				.select((source, name))
				.get_results(&connection)?;
			config.mount_dirs = Some(mount_dirs);
		}

		let found_users: Vec<(String, i32)> = users::table
			.select((users::columns::name, users::columns::admin))
			.get_results(&connection)?;
		config.users = Some(
			found_users
				.into_iter()
				.map(|(name, admin)| ConfigUser {
					name,
					password: "".to_owned(),
					admin: admin != 0,
				})
				.collect::<_>(),
		);

		let ydns = ddns_config
			.select((host, username, password))
			.get_result(&connection)?;
		config.ydns = Some(ydns);

		Ok(config)
	}

	pub fn amend(&self, new_config: &Config) -> anyhow::Result<()> {
		let connection = self.db.connect()?;

		if let Some(ref mount_dirs) = new_config.mount_dirs {
			diesel::delete(mount_points::table).execute(&connection)?;
			diesel::insert_into(mount_points::table)
				.values(mount_dirs)
				.execute(&*connection)?; // TODO https://github.com/diesel-rs/diesel/issues/1822
		}

		if let Some(ref config_users) = new_config.users {
			let old_usernames: Vec<String> =
				users::table.select(users::name).get_results(&connection)?;

			// Delete users that are not in new list
			let delete_usernames: Vec<String> = old_usernames
				.iter()
				.cloned()
				.filter(|old_name| config_users.iter().find(|u| &u.name == old_name).is_none())
				.collect::<_>();
			diesel::delete(users::table.filter(users::name.eq_any(&delete_usernames)))
				.execute(&connection)?;

			// Insert new users
			let insert_users: Vec<&ConfigUser> = config_users
				.iter()
				.filter(|u| {
					!u.name.is_empty()
						&& !u.password.is_empty()
						&& old_usernames
							.iter()
							.find(|old_name| *old_name == &u.name)
							.is_none()
				})
				.collect::<_>();
			for config_user in &insert_users {
				self.user_manager
					.create_user(&config_user.name, &config_user.password)?;
			}

			// Update users
			for user in config_users.iter() {
				// Update password if provided
				if !user.password.is_empty() {
					self.user_manager.set_password(&user.name, &user.password)?;
				}

				// Update admin rights
				diesel::update(users::table.filter(users::name.eq(&user.name)))
					.set(users::admin.eq(user.admin as i32))
					.execute(&connection)?;
			}
		}

		if let Some(sleep_duration) = new_config.reindex_every_n_seconds {
			diesel::update(misc_settings::table)
				.set(misc_settings::index_sleep_duration_seconds.eq(sleep_duration as i32))
				.execute(&connection)?;
		}

		if let Some(ref album_art_pattern) = new_config.album_art_pattern {
			diesel::update(misc_settings::table)
				.set(misc_settings::index_album_art_pattern.eq(album_art_pattern))
				.execute(&connection)?;
		}

		if let Some(ref ydns) = new_config.ydns {
			use self::ddns_config::dsl::*;
			diesel::update(ddns_config)
				.set((
					host.eq(ydns.host.clone()),
					username.eq(ydns.username.clone()),
					password.eq(ydns.password.clone()),
				))
				.execute(&connection)?;
		}

		Ok(())
	}
}
