CREATE TEMPORARY TABLE users_backup(id, name, password_salt, password_hash, admin);
INSERT INTO users_backup SELECT id, name, password_salt, password_hash, admin FROM users;
DROP TABLE users;
CREATE TABLE users (
	id INTEGER PRIMARY KEY NOT NULL,
	name TEXT NOT NULL,
	password_salt BLOB NOT NULL,
	password_hash BLOB NOT NULL,
	admin INTEGER NOT NULL,
	UNIQUE(name)
);
INSERT INTO users SELECT * FROM users_backup;
DROP TABLE users_backup;
