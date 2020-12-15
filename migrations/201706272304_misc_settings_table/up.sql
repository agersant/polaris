CREATE TABLE misc_settings (
	id INTEGER PRIMARY KEY NOT NULL CHECK(id = 0),
	auth_secret TEXT NOT NULL,
	index_sleep_duration_seconds INTEGER NOT NULL,
	index_album_art_pattern TEXT NOT NULL
);
INSERT INTO misc_settings (id, auth_secret, index_sleep_duration_seconds, index_album_art_pattern) VALUES (0, hex(randomblob(64)), 1800, "Folder.(jpeg|jpg|png)");
