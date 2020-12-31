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
	assert_eq!(
		ctx.user_manager.create(&new_user).unwrap_err(),
		Error::EmptyUsername
	);
}

#[test]
fn cannot_create_user_with_blank_password() {
	let ctx = test::ContextBuilder::new(test_name!()).build();
	let new_user = NewUser {
		name: TEST_USERNAME.to_owned(),
		password: "".to_owned(),
		admin: false,
	};
	assert_eq!(
		ctx.user_manager.create(&new_user).unwrap_err(),
		Error::EmptyPassword
	);
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
	assert_eq!(
		ctx.user_manager
			.login(TEST_USERNAME, "not the password")
			.unwrap_err(),
		Error::IncorrectPassword
	)
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
	assert_eq!(
		authorization.unwrap_err(),
		Error::IncorrectAuthorizationScope
	)
}
