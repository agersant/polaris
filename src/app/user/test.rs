use super::*;
use crate::db;
use crate::test_name;

#[test]
fn test_preferences_read_write() {
	let db = db::get_test_db(&test_name!());
	let manager = Manager::new(db);

	let new_preferences = Preferences {
		web_theme_base: Some("very-dark-theme".to_owned()),
		web_theme_accent: Some("#FF0000".to_owned()),
		lastfm_username: None,
	};

	manager.create_user("Walter", "super_secret!").unwrap();

	manager
		.write_preferences("Walter", &new_preferences)
		.unwrap();

	let read_preferences = manager.read_preferences("Walter").unwrap();
	assert_eq!(new_preferences, read_preferences);
}
