CREATE TABLE users (
	id INTEGER PRIMARY KEY NOT NULL,
	name TEXT NOT NULL,
	password_salt BLOB NOT NULL,
	password_hash BLOB NOT NULL,
	UNIQUE(name)
);
