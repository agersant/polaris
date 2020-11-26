CREATE TEMPORARY TABLE misc_settings_backup(id, auth_secret, index_sleep_duration_seconds, index_album_art_pattern);
INSERT INTO misc_settings_backup SELECT id, auth_secret, index_sleep_duration_seconds, index_album_art_pattern FROM misc_settings;
DROP TABLE misc_settings;
CREATE TABLE misc_settings (
	   id INTEGER PRIMARY KEY NOT NULL CHECK(id = 0),
	   auth_secret BLOB NOT NULL DEFAULT (randomblob(32)),
	   index_sleep_duration_seconds INTEGER NOT NULL,
	   index_album_art_pattern TEXT NOT NULL
);
INSERT INTO misc_settings SELECT * FROM misc_settings_backup;
DROP TABLE misc_settings_backup;
