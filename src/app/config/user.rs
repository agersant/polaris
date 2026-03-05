use crate::app::{auth, Error};

use super::storage;
use super::Config;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct User {
	pub name: String,
	pub admin: Option<bool>,
	pub initial_password: Option<String>,
	pub hashed_password: String,
}

impl User {
	pub fn is_admin(&self) -> bool {
		self.admin == Some(true)
	}
}

impl TryFrom<storage::User> for User {
	type Error = Error;

	fn try_from(user: storage::User) -> Result<Self, Self::Error> {
		let hashed_password = match (&user.initial_password, &user.hashed_password) {
			(_, Some(p)) => p.clone(),
			(Some(p), None) => auth::hash_password(p)?,
			(None, None) => return Err(Error::EmptyPassword),
		};

		Ok(Self {
			name: user.name,
			admin: user.admin,
			initial_password: user.initial_password,
			hashed_password,
		})
	}
}

impl From<User> for storage::User {
	fn from(user: User) -> Self {
		Self {
			name: user.name,
			admin: user.admin,
			initial_password: user.initial_password,
			hashed_password: Some(user.hashed_password),
		}
	}
}

impl Config {
	pub fn set_users(&mut self, users: Vec<storage::User>) -> Result<(), Error> {
		let mut new_users = Vec::new();
		for user in users {
			let user = <storage::User as TryInto<User>>::try_into(user)?;
			new_users.push(user);
		}
		new_users.dedup_by(|a, b| a.name == b.name);
		self.users = new_users;
		Ok(())
	}

	pub fn create_user(
		&mut self,
		username: &str,
		password: &str,
		admin: bool,
	) -> Result<(), Error> {
		if username.is_empty() {
			return Err(Error::EmptyUsername);
		}

		if self.exists(username) {
			return Err(Error::DuplicateUsername);
		}

		let password_hash = auth::hash_password(password)?;

		self.users.push(User {
			name: username.to_owned(),
			admin: Some(admin),
			initial_password: None,
			hashed_password: password_hash,
		});

		Ok(())
	}

	pub fn exists(&self, username: &str) -> bool {
		self.users.iter().any(|u| u.name == username)
	}

	pub fn get_user(&self, username: &str) -> Option<&User> {
		self.users.iter().find(|u| u.name == username)
	}

	pub fn get_user_mut(&mut self, username: &str) -> Option<&mut User> {
		self.users.iter_mut().find(|u| u.name == username)
	}

	pub fn authenticate(
		&self,
		auth_token: &auth::Token,
		scope: auth::Scope,
		auth_secret: &auth::Secret,
	) -> Result<auth::Authorization, Error> {
		let authorization = auth::decode_auth_token(auth_token, scope, auth_secret)?;
		if self.exists(&authorization.username) {
			Ok(authorization)
		} else {
			Err(Error::IncorrectUsername)
		}
	}

	pub fn login(
		&self,
		username: &str,
		password: &str,
		auth_secret: &auth::Secret,
	) -> Result<auth::Token, Error> {
		let user = self.get_user(username).ok_or(Error::IncorrectUsername)?;
		if auth::verify_password(&user.hashed_password, password) {
			let authorization = auth::Authorization {
				username: username.to_owned(),
				scope: auth::Scope::PolarisAuth,
			};
			auth::generate_auth_token(&authorization, auth_secret)
		} else {
			Err(Error::IncorrectPassword)
		}
	}

	pub fn set_is_admin(&mut self, username: &str, is_admin: bool) -> Result<(), Error> {
		let user = self.get_user_mut(username).ok_or(Error::UserNotFound)?;
		user.admin = Some(is_admin);
		Ok(())
	}

	pub fn set_password(&mut self, username: &str, password: &str) -> Result<(), Error> {
		let user = self.get_user_mut(username).ok_or(Error::UserNotFound)?;
		user.hashed_password = auth::hash_password(password)?;
		Ok(())
	}

	pub fn delete_user(&mut self, username: &str) {
		self.users.retain(|u| u.name != username);
	}
}

#[cfg(test)]
mod test {
	use crate::app::test;
	use crate::test_name;

	use super::*;

	const TEST_USERNAME: &str = "Walter";
	const TEST_PASSWORD: &str = "super_secret!";

	#[test]
	fn adds_password_hashes() {
		let user_in = storage::User {
			name: TEST_USERNAME.to_owned(),
			initial_password: Some(TEST_PASSWORD.to_owned()),
			..Default::default()
		};

		let user: User = user_in.try_into().unwrap();

		let user_out: storage::User = user.into();

		assert_eq!(user_out.name, TEST_USERNAME);
		assert_eq!(user_out.initial_password, Some(TEST_PASSWORD.to_owned()));
		assert!(user_out.hashed_password.is_some());
	}

	#[test]
	fn preserves_password_hashes() {
		let user_in = storage::User {
			name: TEST_USERNAME.to_owned(),
			hashed_password: Some("hash".to_owned()),
			..Default::default()
		};
		let user: User = user_in.clone().try_into().unwrap();
		let user_out: storage::User = user.into();
		assert_eq!(user_out, user_in);
	}

	#[tokio::test]
	async fn create_delete_user_golden_path() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;

		ctx.config_manager
			.create_user(TEST_USERNAME, TEST_PASSWORD, false)
			.await
			.unwrap();
		assert!(ctx.config_manager.get_user(TEST_USERNAME).await.is_ok());

		ctx.config_manager.delete_user(TEST_USERNAME).await.unwrap();
		assert!(ctx.config_manager.get_user(TEST_USERNAME).await.is_err());
	}

	#[tokio::test]
	async fn cannot_create_user_with_blank_username() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;
		let result = ctx.config_manager.create_user("", TEST_PASSWORD, false);
		assert!(matches!(result.await.unwrap_err(), Error::EmptyUsername));
	}

	#[tokio::test]
	async fn cannot_create_user_with_blank_password() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;
		let result = ctx.config_manager.create_user(TEST_USERNAME, "", false);
		assert!(matches!(result.await.unwrap_err(), Error::EmptyPassword));
	}

	#[tokio::test]
	async fn cannot_create_duplicate_user() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;
		let result = ctx
			.config_manager
			.create_user(TEST_USERNAME, TEST_PASSWORD, false);
		assert!(result.await.is_ok());

		let result = ctx
			.config_manager
			.create_user(TEST_USERNAME, TEST_PASSWORD, false);
		assert!(matches!(
			result.await.unwrap_err(),
			Error::DuplicateUsername
		));
	}

	#[tokio::test]
	async fn login_rejects_bad_password() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;

		ctx.config_manager
			.create_user(TEST_USERNAME, TEST_PASSWORD, false)
			.await
			.unwrap();

		let result = ctx.config_manager.login(TEST_USERNAME, "not the password");
		assert!(matches!(
			result.await.unwrap_err(),
			Error::IncorrectPassword
		));
	}

	#[tokio::test]
	async fn login_golden_path() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;

		ctx.config_manager
			.create_user(TEST_USERNAME, TEST_PASSWORD, false)
			.await
			.unwrap();

		let result = ctx.config_manager.login(TEST_USERNAME, TEST_PASSWORD);
		assert!(result.await.is_ok());
	}

	#[tokio::test]
	async fn authenticate_rejects_bad_token() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;

		ctx.config_manager
			.create_user(TEST_USERNAME, TEST_PASSWORD, false)
			.await
			.unwrap();

		let fake_token = auth::Token("fake token".to_owned());
		assert!(ctx
			.config_manager
			.authenticate(&fake_token, auth::Scope::PolarisAuth)
			.await
			.is_err())
	}

	#[tokio::test]
	async fn authenticate_golden_path() {
		let ctx = test::ContextBuilder::new(test_name!()).build().await;

		ctx.config_manager
			.create_user(TEST_USERNAME, TEST_PASSWORD, false)
			.await
			.unwrap();

		let token = ctx
			.config_manager
			.login(TEST_USERNAME, TEST_PASSWORD)
			.await
			.unwrap();

		let authorization = ctx
			.config_manager
			.authenticate(&token, auth::Scope::PolarisAuth)
			.await
			.unwrap();

		assert_eq!(
			authorization,
			auth::Authorization {
				username: TEST_USERNAME.to_owned(),
				scope: auth::Scope::PolarisAuth,
			}
		)
	}
}
