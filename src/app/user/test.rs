use super::*;
use crate::db;
use crate::test_name;

#[test]
fn test_preferences_read_write() {
	let db = db::get_test_db(&test_name!());
	let user_manager = Manager::new(db);

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

// TODO test cannot create user with blank username
// TODO test cannot create user with blank password
