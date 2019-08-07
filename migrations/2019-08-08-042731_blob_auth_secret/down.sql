CREATE TEMPORARY TABLE misc_settings_backup(id, index_sleep_duration_seconds, index_album_art_pattern, prefix_url);
INSERT INTO misc_settings_backup
SELECT id, index_sleep_duration_seconds, index_album_art_pattern, prefix_url
FROM misc_settings;
DROP TABLE misc_settings;
CREATE TABLE misc_settings (
	   id INTEGER PRIMARY KEY NOT NULL CHECK(id = 0),
	   auth_secret BLOB NOT NULL DEFAULT (hex(randomblob(32))),
	   index_sleep_duration_seconds INTEGER NOT NULL,
	   index_album_art_pattern TEXT NOT NULL,
       prefix_url TEXT NOT NULL DEFAULT ""
);
INSERT INTO misc_settings(id, index_sleep_duration_seconds, index_album_art_pattern, prefix_url)
SELECT * FROM misc_settings_backup;
DROP TABLE misc_settings_backup;
