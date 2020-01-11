CREATE TEMPORARY TABLE users_backup(id, name, password_hash, admin, lastfm_username, lastfm_session_key);
INSERT INTO users_backup SELECT id, name, password_hash, admin, lastfm_username, lastfm_session_key FROM users;
DROP TABLE users;
CREATE TABLE users (
	id INTEGER PRIMARY KEY NOT NULL,
	name TEXT NOT NULL,
	password_hash TEXT NOT NULL,
	admin INTEGER NOT NULL,
	lastfm_username TEXT,
	lastfm_session_key TEXT,
	UNIQUE(name)
);
INSERT INTO users SELECT * FROM users_backup;
DROP TABLE users_backup;
