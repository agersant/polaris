use super::*;
use crate::app::settings;
use crate::db::DB;
use crate::test_name;

#[cfg(test)]
pub fn get_test_db(name: &str) -> DB {
	let mut db_path = std::path::PathBuf::new();
	db_path.push("test-output");
	std::fs::create_dir_all(&db_path).unwrap();

	db_path.push(name);
	if db_path.exists() {
		std::fs::remove_file(&db_path).unwrap();
	}

	DB::new(&db_path).unwrap()
}

#[test]
fn create_delete_user_golden_path() {
	let db = get_test_db(&test_name!());
	let settings_manager = settings::Manager::new(db.clone());
	let auth_secret = settings_manager.get_auth_secret().unwrap();
	let user_manager = Manager::new(db, auth_secret);

	let new_user = NewUser {
		name: "Walter".to_owned(),
		password: "super_secret!".to_owned(),
		admin: false,
	};

	assert_eq!(user_manager.list().unwrap().len(), 0);
	user_manager.create(&new_user).unwrap();
	assert_eq!(user_manager.list().unwrap().len(), 1);
	user_manager.delete(&new_user.name).unwrap();
	assert_eq!(user_manager.list().unwrap().len(), 0);
}

#[test]
fn cannot_create_user_with_blank_username() {
	let db = get_test_db(&test_name!());
	let settings_manager = settings::Manager::new(db.clone());
	let auth_secret = settings_manager.get_auth_secret().unwrap();
	let user_manager = Manager::new(db, auth_secret);

	let new_user = NewUser {
		name: "".to_owned(),
		password: "super_secret!".to_owned(),
		admin: false,
	};

	assert_eq!(
		user_manager.create(&new_user).unwrap_err(),
		Error::EmptyUsername
	);
}

#[test]
fn cannot_create_user_with_blank_password() {
	let db = get_test_db(&test_name!());
	let settings_manager = settings::Manager::new(db.clone());
	let auth_secret = settings_manager.get_auth_secret().unwrap();
	let user_manager = Manager::new(db, auth_secret);

	let new_user = NewUser {
		name: "Walter".to_owned(),
		password: "".to_owned(),
		admin: false,
	};

	assert_eq!(
		user_manager.create(&new_user).unwrap_err(),
		Error::EmptyPassword
	);
}

#[test]
fn cannot_create_duplicate_user() {
	let db = get_test_db(&test_name!());
	let settings_manager = settings::Manager::new(db.clone());
	let auth_secret = settings_manager.get_auth_secret().unwrap();
	let user_manager = Manager::new(db, auth_secret);

	let new_user = NewUser {
		name: "Walter".to_owned(),
		password: "super_secret!".to_owned(),
		admin: false,
	};

	user_manager.create(&new_user).unwrap();
	user_manager.create(&new_user).unwrap_err();
}

#[test]
fn can_read_write_preferences() {
	let db = get_test_db(&test_name!());
	let settings_manager = settings::Manager::new(db.clone());
	let auth_secret = settings_manager.get_auth_secret().unwrap();
	let user_manager = Manager::new(db, auth_secret);

	let new_preferences = Preferences {
		web_theme_base: Some("very-dark-theme".to_owned()),
		web_theme_accent: Some("#FF0000".to_owned()),
		lastfm_username: None,
	};

	let new_user = NewUser {
		name: "Walter".to_owned(),
		password: "super_secret!".to_owned(),
		admin: false,
	};
	user_manager.create(&new_user).unwrap();

	user_manager
		.write_preferences("Walter", &new_preferences)
		.unwrap();

	let read_preferences = user_manager.read_preferences("Walter").unwrap();
	assert_eq!(new_preferences, read_preferences);
}

#[test]
fn login_rejects_bad_password() {
	let db = get_test_db(&test_name!());
	let settings_manager = settings::Manager::new(db.clone());
	let auth_secret = settings_manager.get_auth_secret().unwrap();
	let user_manager = Manager::new(db, auth_secret);

	let username = "Walter";
	let password = "super_secret!";

	let new_user = NewUser {
		name: username.to_owned(),
		password: password.to_owned(),
		admin: false,
	};

	user_manager.create(&new_user).unwrap();
	assert_eq!(
		user_manager
			.login(username, "not the password")
			.unwrap_err(),
		Error::IncorrectPassword
	)
}

#[test]
fn login_golden_path() {
	let db = get_test_db(&test_name!());
	let settings_manager = settings::Manager::new(db.clone());
	let auth_secret = settings_manager.get_auth_secret().unwrap();
	let user_manager = Manager::new(db, auth_secret);

	let username = "Walter";
	let password = "super_secret!";

	let new_user = NewUser {
		name: username.to_owned(),
		password: password.to_owned(),
		admin: false,
	};

	user_manager.create(&new_user).unwrap();
	assert!(user_manager.login(username, password).is_ok())
}

#[test]
fn authenticate_rejects_bad_token() {
	let db = get_test_db(&test_name!());
	let settings_manager = settings::Manager::new(db.clone());
	let auth_secret = settings_manager.get_auth_secret().unwrap();
	let user_manager = Manager::new(db, auth_secret);

	let username = "Walter";
	let password = "super_secret!";

	let new_user = NewUser {
		name: username.to_owned(),
		password: password.to_owned(),
		admin: false,
	};

	user_manager.create(&new_user).unwrap();
	let token = AuthToken {
		data: "fake token".to_owned(),
	};
	assert!(user_manager.authenticate(&token).is_err())
}

#[test]
fn authenticate_golden_path() {
	let db = get_test_db(&test_name!());
	let settings_manager = settings::Manager::new(db.clone());
	let auth_secret = settings_manager.get_auth_secret().unwrap();
	let user_manager = Manager::new(db, auth_secret);

	let username = "Walter";
	let password = "super_secret!";

	let new_user = NewUser {
		name: username.to_owned(),
		password: password.to_owned(),
		admin: false,
	};

	user_manager.create(&new_user).unwrap();
	let token = user_manager.login(username, password).unwrap();
	assert_eq!(user_manager.authenticate(&token).unwrap(), username)
}
