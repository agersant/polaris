use pbkdf2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use pbkdf2::Pbkdf2;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::app::settings::AuthSecret;
use crate::db::{self, DB};

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error(transparent)]
	Database(#[from] sqlx::Error),
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

#[derive(Debug)]
pub struct User {
	pub name: String,
	pub admin: i64,
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

	pub async fn create(&self, new_user: &NewUser) -> Result<(), Error> {
		if new_user.name.is_empty() {
			return Err(Error::EmptyUsername);
		}

		let password_hash = hash_password(&new_user.password)?;

		sqlx::query!(
			"INSERT INTO users (name, password_hash, admin) VALUES($1, $2, $3)",
			new_user.name,
			password_hash,
			new_user.admin
		)
		.execute(self.db.connect().await?.as_mut())
		.await?;

		Ok(())
	}

	pub async fn delete(&self, username: &str) -> Result<(), Error> {
		sqlx::query!("DELETE FROM users WHERE name = $1", username)
			.execute(self.db.connect().await?.as_mut())
			.await?;
		Ok(())
	}

	pub async fn set_password(&self, username: &str, password: &str) -> Result<(), Error> {
		let hash = hash_password(password)?;
		sqlx::query!(
			"UPDATE users SET password_hash = $1 WHERE name = $2",
			hash,
			username
		)
		.execute(self.db.connect().await?.as_mut())
		.await?;
		Ok(())
	}

	pub async fn set_is_admin(&self, username: &str, is_admin: bool) -> Result<(), Error> {
		sqlx::query!(
			"UPDATE users SET admin = $1 WHERE name = $2",
			is_admin,
			username
		)
		.execute(self.db.connect().await?.as_mut())
		.await?;
		Ok(())
	}

	pub async fn login(&self, username: &str, password: &str) -> Result<AuthToken, Error> {
		match sqlx::query_scalar!("SELECT password_hash FROM users WHERE name = $1", username)
			.fetch_optional(self.db.connect().await?.as_mut())
			.await?
		{
			None => Err(Error::IncorrectUsername),
			Some(hash) => {
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
		}
	}

	pub async fn authenticate(
		&self,
		auth_token: &AuthToken,
		scope: AuthorizationScope,
	) -> Result<Authorization, Error> {
		let authorization = self.decode_auth_token(auth_token, scope)?;
		if self.exists(&authorization.username).await? {
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

	pub async fn count(&self) -> Result<i32, Error> {
		let count = sqlx::query_scalar!("SELECT COUNT(*) FROM users")
			.fetch_one(self.db.connect().await?.as_mut())
			.await?;
		Ok(count)
	}

	pub async fn list(&self) -> Result<Vec<User>, Error> {
		let listed_users = sqlx::query_as!(User, "SELECT name, admin FROM users")
			.fetch_all(self.db.connect().await?.as_mut())
			.await?;
		Ok(listed_users)
	}

	pub async fn exists(&self, username: &str) -> Result<bool, Error> {
		Ok(
			0 < sqlx::query_scalar!("SELECT COUNT(*) FROM users WHERE name = $1", username)
				.fetch_one(self.db.connect().await?.as_mut())
				.await?,
		)
	}

	pub async fn is_admin(&self, username: &str) -> Result<bool, Error> {
		Ok(
			0 < sqlx::query_scalar!("SELECT admin FROM users WHERE name = $1", username)
				.fetch_one(self.db.connect().await?.as_mut())
				.await?,
		)
	}

	pub async fn read_preferences(&self, username: &str) -> Result<Preferences, Error> {
		Ok(sqlx::query_as!(
			Preferences,
			"SELECT web_theme_base, web_theme_accent, lastfm_username FROM users WHERE name = $1",
			username
		)
		.fetch_one(self.db.connect().await?.as_mut())
		.await?)
	}

	pub async fn write_preferences(
		&self,
		username: &str,
		preferences: &Preferences,
	) -> Result<(), Error> {
		sqlx::query!(
			"UPDATE users SET web_theme_base = $1, web_theme_accent = $2 WHERE name = $3",
			preferences.web_theme_base,
			preferences.web_theme_accent,
			username
		)
		.execute(self.db.connect().await?.as_mut())
		.await?;
		Ok(())
	}

	pub async fn lastfm_link(
		&self,
		username: &str,
		lastfm_login: &str,
		session_key: &str,
	) -> Result<(), Error> {
		sqlx::query!(
			"UPDATE users SET lastfm_username = $1, lastfm_session_key = $2 WHERE name = $3",
			lastfm_login,
			session_key,
			username
		)
		.execute(self.db.connect().await?.as_mut())
		.await?;
		Ok(())
	}

	pub fn generate_lastfm_link_token(&self, username: &str) -> Result<AuthToken, Error> {
		self.generate_auth_token(&Authorization {
			username: username.to_owned(),
			scope: AuthorizationScope::LastFMLink,
		})
	}

	pub async fn get_lastfm_session_key(&self, username: &str) -> Result<String, Error> {
		let token: Option<String> = sqlx::query_scalar!(
			"SELECT lastfm_session_key FROM users WHERE name = $1",
			username
		)
		.fetch_one(self.db.connect().await?.as_mut())
		.await?;
		token.ok_or(Error::MissingLastFMSessionKey)
	}

	pub async fn is_lastfm_linked(&self, username: &str) -> bool {
		self.get_lastfm_session_key(username).await.is_ok()
	}

	pub async fn lastfm_unlink(&self, username: &str) -> Result<(), Error> {
		let null: Option<String> = None;
		sqlx::query!(
			"UPDATE users SET lastfm_session_key = $1, lastfm_username = $1 WHERE name = $2",
			null,
			username
		)
		.execute(self.db.connect().await?.as_mut())
		.await?;
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

	#[tokio::test]
	async fn create_delete_user_golden_path() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;

		let new_user = NewUser {
			name: TEST_USERNAME.to_owned(),
			password: TEST_PASSWORD.to_owned(),
			admin: false,
		};

		ctx.user_manager.create(&new_user).await.unwrap();
		assert_eq!(ctx.user_manager.list().await.unwrap().len(), 1);

		ctx.user_manager.delete(&new_user.name).await.unwrap();
		assert_eq!(ctx.user_manager.list().await.unwrap().len(), 0);
	}

	#[tokio::test]
	async fn cannot_create_user_with_blank_username() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;
		let new_user = NewUser {
			name: "".to_owned(),
			password: TEST_PASSWORD.to_owned(),
			admin: false,
		};
		assert!(matches!(
			ctx.user_manager.create(&new_user).await.unwrap_err(),
			Error::EmptyUsername
		));
	}

	#[tokio::test]
	async fn cannot_create_user_with_blank_password() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;
		let new_user = NewUser {
			name: TEST_USERNAME.to_owned(),
			password: "".to_owned(),
			admin: false,
		};
		assert!(matches!(
			ctx.user_manager.create(&new_user).await.unwrap_err(),
			Error::EmptyPassword
		));
	}

	#[tokio::test]
	async fn cannot_create_duplicate_user() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;
		let new_user = NewUser {
			name: TEST_USERNAME.to_owned(),
			password: TEST_PASSWORD.to_owned(),
			admin: false,
		};
		ctx.user_manager.create(&new_user).await.unwrap();
		ctx.user_manager.create(&new_user).await.unwrap_err();
	}

	#[tokio::test]
	async fn can_read_write_preferences() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;

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
		ctx.user_manager.create(&new_user).await.unwrap();

		ctx.user_manager
			.write_preferences(TEST_USERNAME, &new_preferences)
			.await
			.unwrap();

		let read_preferences = ctx.user_manager.read_preferences("Walter").await.unwrap();
		assert_eq!(new_preferences, read_preferences);
	}

	#[tokio::test]
	async fn login_rejects_bad_password() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;

		let new_user = NewUser {
			name: TEST_USERNAME.to_owned(),
			password: TEST_PASSWORD.to_owned(),
			admin: false,
		};

		ctx.user_manager.create(&new_user).await.unwrap();
		assert!(matches!(
			ctx.user_manager
				.login(TEST_USERNAME, "not the password")
				.await
				.unwrap_err(),
			Error::IncorrectPassword
		));
	}

	#[tokio::test]
	async fn login_golden_path() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;
		let new_user = NewUser {
			name: TEST_USERNAME.to_owned(),
			password: TEST_PASSWORD.to_owned(),
			admin: false,
		};
		ctx.user_manager.create(&new_user).await.unwrap();
		assert!(ctx
			.user_manager
			.login(TEST_USERNAME, TEST_PASSWORD)
			.await
			.is_ok())
	}

	#[tokio::test]
	async fn authenticate_rejects_bad_token() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;

		let new_user = NewUser {
			name: TEST_USERNAME.to_owned(),
			password: TEST_PASSWORD.to_owned(),
			admin: false,
		};

		ctx.user_manager.create(&new_user).await.unwrap();
		let fake_token = AuthToken("fake token".to_owned());
		assert!(ctx
			.user_manager
			.authenticate(&fake_token, AuthorizationScope::PolarisAuth)
			.await
			.is_err())
	}

	#[tokio::test]
	async fn authenticate_golden_path() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;

		let new_user = NewUser {
			name: TEST_USERNAME.to_owned(),
			password: TEST_PASSWORD.to_owned(),
			admin: false,
		};

		ctx.user_manager.create(&new_user).await.unwrap();
		let token = ctx
			.user_manager
			.login(TEST_USERNAME, TEST_PASSWORD)
			.await
			.unwrap();
		let authorization = ctx
			.user_manager
			.authenticate(&token, AuthorizationScope::PolarisAuth)
			.await
			.unwrap();
		assert_eq!(
			authorization,
			Authorization {
				username: TEST_USERNAME.to_owned(),
				scope: AuthorizationScope::PolarisAuth,
			}
		)
	}

	#[tokio::test]
	async fn authenticate_validates_scope() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;

		let new_user = NewUser {
			name: TEST_USERNAME.to_owned(),
			password: TEST_PASSWORD.to_owned(),
			admin: false,
		};

		ctx.user_manager.create(&new_user).await.unwrap();
		let token = ctx
			.user_manager
			.generate_lastfm_link_token(TEST_USERNAME)
			.unwrap();
		let authorization = ctx
			.user_manager
			.authenticate(&token, AuthorizationScope::PolarisAuth)
			.await;
		assert!(matches!(
			authorization.unwrap_err(),
			Error::IncorrectAuthorizationScope
		));
	}
}
