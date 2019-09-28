DROP TABLE users;
CREATE TABLE users (
	id INTEGER PRIMARY KEY NOT NULL,
	name TEXT NOT NULL,
	password_salt BLOB NOT NULL,
	password_hash BLOB NOT NULL,
	admin INTEGER NOT NULL,
	lastfm_username TEXT,
	lastfm_session_key TEXT,
	UNIQUE(name)
);
