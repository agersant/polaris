use diesel::prelude::*;
use pbkdf2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use pbkdf2::Pbkdf2;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::app::settings::AuthSecret;
use crate::db::{self, users, DB};

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error(transparent)]
	Database(#[from] diesel::result::Error),
	#[error(transparent)]
	DatabaseConnection(#[from] db::Error),
	#[error("Cannot use empty username")]
	EmptyUsername,
	#[error("Cannot use empty password")]
	EmptyPassword,
	#[error("Username does not exist")]
	IncorrectUsername,
	#[error("Password does not match username")]
	IncorrectPassword,
	#[error("Invalid auth token")]
	InvalidAuthToken,
	#[error("Incorrect authorization scope")]
	IncorrectAuthorizationScope,
	#[error("Last.fm session key is missing")]
	MissingLastFMSessionKey,
	#[error("Failed to hash password")]
	PasswordHashing,
	#[error("Failed to encode authorization token")]
	AuthorizationTokenEncoding,
	#[error("Failed to encode Branca token")]
	BrancaTokenEncoding,
}

#[derive(Debug, Insertable, Queryable)]
#[diesel(table_name = users)]
pub struct User {
	pub name: String,
	pub password_hash: String,
	pub admin: i32,
}

impl User {
	pub fn is_admin(&self) -> bool {
		self.admin != 0
	}
}

#[derive(Debug, Deserialize)]
pub struct NewUser {
	pub name: String,
	pub password: String,
	pub admin: bool,
}

#[derive(Debug)]
pub struct AuthToken(pub String);

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum AuthorizationScope {
	PolarisAuth,
	LastFMLink,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Authorization {
	pub username: String,
	pub scope: AuthorizationScope,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preferences {
	pub lastfm_username: Option<String>,
	pub web_theme_base: Option<String>,
	pub web_theme_accent: Option<String>,
}

#[derive(Clone)]
pub struct Manager {
	db: DB,
	auth_secret: AuthSecret,
}

impl Manager {
	pub fn new(db: DB, auth_secret: AuthSecret) -> Self {
		Self { db, auth_secret }
	}

	pub fn create(&self, new_user: &NewUser) -> Result<(), Error> {
		if new_user.name.is_empty() {
			return Err(Error::EmptyUsername);
		}

		let password_hash = hash_password(&new_user.password)?;
		let mut connection = self.db.connect()?;
		let new_user = User {
			name: new_user.name.to_owned(),
			password_hash,
			admin: new_user.admin as i32,
		};

		diesel::insert_into(users::table)
			.values(&new_user)
			.execute(&mut connection)?;
		Ok(())
	}

	pub fn delete(&self, username: &str) -> Result<(), Error> {
		use crate::db::users::dsl::*;
		let mut connection = self.db.connect()?;
		diesel::delete(users.filter(name.eq(username))).execute(&mut connection)?;
		Ok(())
	}

	pub fn set_password(&self, username: &str, password: &str) -> Result<(), Error> {
		let hash = hash_password(password)?;
		let mut connection = self.db.connect()?;
		use crate::db::users::dsl::*;
		diesel::update(users.filter(name.eq(username)))
			.set(password_hash.eq(hash))
			.execute(&mut connection)?;
		Ok(())
	}

	pub fn set_is_admin(&self, username: &str, is_admin: bool) -> Result<(), Error> {
		use crate::db::users::dsl::*;
		let mut connection = self.db.connect()?;
		diesel::update(users.filter(name.eq(username)))
			.set(admin.eq(is_admin as i32))
			.execute(&mut connection)?;
		Ok(())
	}

	pub fn login(&self, username: &str, password: &str) -> Result<AuthToken, Error> {
		use crate::db::users::dsl::*;
		let mut connection = self.db.connect()?;
		match users
			.select(password_hash)
			.filter(name.eq(username))
			.get_result(&mut connection)
		{
			Err(diesel::result::Error::NotFound) => Err(Error::IncorrectUsername),
			Ok(hash) => {
				let hash: String = hash;
				if verify_password(&hash, password) {
					let authorization = Authorization {
						username: username.to_owned(),
						scope: AuthorizationScope::PolarisAuth,
					};
					self.generate_auth_token(&authorization)
				} else {
					Err(Error::IncorrectPassword)
				}
			}
			Err(e) => Err(e.into()),
		}
	}

	pub fn authenticate(
		&self,
		auth_token: &AuthToken,
		scope: AuthorizationScope,
	) -> Result<Authorization, Error> {
		let authorization = self.decode_auth_token(auth_token, scope)?;
		if self.exists(&authorization.username)? {
			Ok(authorization)
		} else {
			Err(Error::IncorrectUsername)
		}
	}

	fn decode_auth_token(
		&self,
		auth_token: &AuthToken,
		scope: AuthorizationScope,
	) -> Result<Authorization, Error> {
		let AuthToken(data) = auth_token;
		let ttl = match scope {
			AuthorizationScope::PolarisAuth => 0,      // permanent
			AuthorizationScope::LastFMLink => 10 * 60, // 10 minutes
		};
		let authorization = branca::decode(data, &self.auth_secret.key, ttl)
			.map_err(|_| Error::InvalidAuthToken)?;
		let authorization: Authorization =
			serde_json::from_slice(&authorization[..]).map_err(|_| Error::InvalidAuthToken)?;
		if authorization.scope != scope {
			return Err(Error::IncorrectAuthorizationScope);
		}
		Ok(authorization)
	}

	fn generate_auth_token(&self, authorization: &Authorization) -> Result<AuthToken, Error> {
		let serialized_authorization =
			serde_json::to_string(&authorization).or(Err(Error::AuthorizationTokenEncoding))?;
		branca::encode(
			serialized_authorization.as_bytes(),
			&self.auth_secret.key,
			SystemTime::now()
				.duration_since(UNIX_EPOCH)
				.unwrap_or_default()
				.as_secs() as u32,
		)
		.or(Err(Error::BrancaTokenEncoding))
		.map(AuthToken)
	}

	pub fn count(&self) -> Result<i64, Error> {
		use crate::db::users::dsl::*;
		let mut connection = self.db.connect()?;
		let count = users.count().get_result(&mut connection)?;
		Ok(count)
	}

	pub fn list(&self) -> Result<Vec<User>, Error> {
		use crate::db::users::dsl::*;
		let mut connection = self.db.connect()?;
		let listed_users = users
			.select((name, password_hash, admin))
			.get_results(&mut connection)?;
		Ok(listed_users)
	}

	pub fn exists(&self, username: &str) -> Result<bool, Error> {
		use crate::db::users::dsl::*;
		let mut connection = self.db.connect()?;
		let results: Vec<String> = users
			.select(name)
			.filter(name.eq(username))
			.get_results(&mut connection)?;
		Ok(!results.is_empty())
	}

	pub fn is_admin(&self, username: &str) -> Result<bool, Error> {
		use crate::db::users::dsl::*;
		let mut connection = self.db.connect()?;
		let is_admin: i32 = users
			.filter(name.eq(username))
			.select(admin)
			.get_result(&mut connection)?;
		Ok(is_admin != 0)
	}

	pub fn read_preferences(&self, username: &str) -> Result<Preferences, Error> {
		use crate::db::users::dsl::*;
		let mut connection = self.db.connect()?;
		let (theme_base, theme_accent, read_lastfm_username) = users
			.select((web_theme_base, web_theme_accent, lastfm_username))
			.filter(name.eq(username))
			.get_result(&mut connection)?;
		Ok(Preferences {
			web_theme_base: theme_base,
			web_theme_accent: theme_accent,
			lastfm_username: read_lastfm_username,
		})
	}

	pub fn write_preferences(
		&self,
		username: &str,
		preferences: &Preferences,
	) -> Result<(), Error> {
		use crate::db::users::dsl::*;
		let mut connection = self.db.connect()?;
		diesel::update(users.filter(name.eq(username)))
			.set((
				web_theme_base.eq(&preferences.web_theme_base),
				web_theme_accent.eq(&preferences.web_theme_accent),
			))
			.execute(&mut connection)?;
		Ok(())
	}

	pub fn lastfm_link(
		&self,
		username: &str,
		lastfm_login: &str,
		session_key: &str,
	) -> Result<(), Error> {
		use crate::db::users::dsl::*;
		let mut connection = self.db.connect()?;
		diesel::update(users.filter(name.eq(username)))
			.set((
				lastfm_username.eq(lastfm_login),
				lastfm_session_key.eq(session_key),
			))
			.execute(&mut connection)?;
		Ok(())
	}

	pub fn generate_lastfm_link_token(&self, username: &str) -> Result<AuthToken, Error> {
		self.generate_auth_token(&Authorization {
			username: username.to_owned(),
			scope: AuthorizationScope::LastFMLink,
		})
	}

	pub fn get_lastfm_session_key(&self, username: &str) -> Result<String, Error> {
		use crate::db::users::dsl::*;
		let mut connection = self.db.connect()?;
		let token: Option<String> = users
			.filter(name.eq(username))
			.select(lastfm_session_key)
			.get_result(&mut connection)?;
		token.ok_or(Error::MissingLastFMSessionKey)
	}

	pub fn is_lastfm_linked(&self, username: &str) -> bool {
		self.get_lastfm_session_key(username).is_ok()
	}

	pub fn lastfm_unlink(&self, username: &str) -> Result<(), Error> {
		use crate::db::users::dsl::*;
		let mut connection = self.db.connect()?;
		let null: Option<String> = None;
		diesel::update(users.filter(name.eq(username)))
			.set((lastfm_session_key.eq(&null), lastfm_username.eq(&null)))
			.execute(&mut connection)?;
		Ok(())
	}
}

fn hash_password(password: &str) -> Result<String, Error> {
	if password.is_empty() {
		return Err(Error::EmptyPassword);
	}
	let salt = SaltString::generate(&mut OsRng);
	match Pbkdf2.hash_password(password.as_bytes(), &salt) {
		Ok(h) => Ok(h.to_string()),
		Err(_) => Err(Error::PasswordHashing),
	}
}

fn verify_password(password_hash: &str, attempted_password: &str) -> bool {
	match PasswordHash::new(password_hash) {
		Ok(h) => Pbkdf2
			.verify_password(attempted_password.as_bytes(), &h)
			.is_ok(),
		Err(_) => false,
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::app::test;
	use crate::test_name;

	const TEST_USERNAME: &str = "Walter";
	const TEST_PASSWORD: &str = "super_secret!";

	#[test]
	fn create_delete_user_golden_path() {
		let ctx = test::ContextBuilder::new(test_name!()).build();

		let new_user = NewUser {
			name: TEST_USERNAME.to_owned(),
			password: TEST_PASSWORD.to_owned(),
			admin: false,
		};

		ctx.user_manager.create(&new_user).unwrap();
		assert_eq!(ctx.user_manager.list().unwrap().len(), 1);

		ctx.user_manager.delete(&new_user.name).unwrap();
		assert_eq!(ctx.user_manager.list().unwrap().len(), 0);
	}

	#[test]
	fn cannot_create_user_with_blank_username() {
		let ctx = test::ContextBuilder::new(test_name!()).build();
		let new_user = NewUser {
			name: "".to_owned(),
			password: TEST_PASSWORD.to_owned(),
			admin: false,
		};
		assert!(matches!(
			ctx.user_manager.create(&new_user).unwrap_err(),
			Error::EmptyUsername
		));
	}

	#[test]
	fn cannot_create_user_with_blank_password() {
		let ctx = test::ContextBuilder::new(test_name!()).build();
		let new_user = NewUser {
			name: TEST_USERNAME.to_owned(),
			password: "".to_owned(),
			admin: false,
		};
		assert!(matches!(
			ctx.user_manager.create(&new_user).unwrap_err(),
			Error::EmptyPassword
		));
	}

	#[test]
	fn cannot_create_duplicate_user() {
		let ctx = test::ContextBuilder::new(test_name!()).build();
		let new_user = NewUser {
			name: TEST_USERNAME.to_owned(),
			password: TEST_PASSWORD.to_owned(),
			admin: false,
		};
		ctx.user_manager.create(&new_user).unwrap();
		ctx.user_manager.create(&new_user).unwrap_err();
	}

	#[test]
	fn can_read_write_preferences() {
		let ctx = test::ContextBuilder::new(test_name!()).build();

		let new_preferences = Preferences {
			web_theme_base: Some("very-dark-theme".to_owned()),
			web_theme_accent: Some("#FF0000".to_owned()),
			lastfm_username: None,
		};

		let new_user = NewUser {
			name: TEST_USERNAME.to_owned(),
			password: TEST_PASSWORD.to_owned(),
			admin: false,
		};
		ctx.user_manager.create(&new_user).unwrap();

		ctx.user_manager
			.write_preferences(TEST_USERNAME, &new_preferences)
			.unwrap();

		let read_preferences = ctx.user_manager.read_preferences("Walter").unwrap();
		assert_eq!(new_preferences, read_preferences);
	}

	#[test]
	fn login_rejects_bad_password() {
		let ctx = test::ContextBuilder::new(test_name!()).build();

		let new_user = NewUser {
			name: TEST_USERNAME.to_owned(),
			password: TEST_PASSWORD.to_owned(),
			admin: false,
		};

		ctx.user_manager.create(&new_user).unwrap();
		assert!(matches!(
			ctx.user_manager
				.login(TEST_USERNAME, "not the password")
				.unwrap_err(),
			Error::IncorrectPassword
		));
	}

	#[test]
	fn login_golden_path() {
		let ctx = test::ContextBuilder::new(test_name!()).build();
		let new_user = NewUser {
			name: TEST_USERNAME.to_owned(),
			password: TEST_PASSWORD.to_owned(),
			admin: false,
		};
		ctx.user_manager.create(&new_user).unwrap();
		assert!(ctx.user_manager.login(TEST_USERNAME, TEST_PASSWORD).is_ok())
	}

	#[test]
	fn authenticate_rejects_bad_token() {
		let ctx = test::ContextBuilder::new(test_name!()).build();

		let new_user = NewUser {
			name: TEST_USERNAME.to_owned(),
			password: TEST_PASSWORD.to_owned(),
			admin: false,
		};

		ctx.user_manager.create(&new_user).unwrap();
		let fake_token = AuthToken("fake token".to_owned());
		assert!(ctx
			.user_manager
			.authenticate(&fake_token, AuthorizationScope::PolarisAuth)
			.is_err())
	}

	#[test]
	fn authenticate_golden_path() {
		let ctx = test::ContextBuilder::new(test_name!()).build();

		let new_user = NewUser {
			name: TEST_USERNAME.to_owned(),
			password: TEST_PASSWORD.to_owned(),
			admin: false,
		};

		ctx.user_manager.create(&new_user).unwrap();
		let token = ctx
			.user_manager
			.login(TEST_USERNAME, TEST_PASSWORD)
			.unwrap();
		let authorization = ctx
			.user_manager
			.authenticate(&token, AuthorizationScope::PolarisAuth)
			.unwrap();
		assert_eq!(
			authorization,
			Authorization {
				username: TEST_USERNAME.to_owned(),
				scope: AuthorizationScope::PolarisAuth,
			}
		)
	}

	#[test]
	fn authenticate_validates_scope() {
		let ctx = test::ContextBuilder::new(test_name!()).build();

		let new_user = NewUser {
			name: TEST_USERNAME.to_owned(),
			password: TEST_PASSWORD.to_owned(),
			admin: false,
		};

		ctx.user_manager.create(&new_user).unwrap();
		let token = ctx
			.user_manager
			.generate_lastfm_link_token(TEST_USERNAME)
			.unwrap();
		let authorization = ctx
			.user_manager
			.authenticate(&token, AuthorizationScope::PolarisAuth);
		assert!(matches!(
			authorization.unwrap_err(),
			Error::IncorrectAuthorizationScope
		));
	}
}
