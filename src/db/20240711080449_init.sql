CREATE TABLE config (
	id INTEGER PRIMARY KEY NOT NULL CHECK(id = 0),
	auth_secret BLOB NOT NULL DEFAULT (randomblob(32)),
	index_sleep_duration_seconds INTEGER NOT NULL,
	index_album_art_pattern TEXT NOT NULL,
	ddns_host TEXT NOT NULL,
	ddns_username TEXT NOT NULL,
	ddns_password TEXT NOT NULL
);

INSERT INTO config (
	id,
	auth_secret,
	index_sleep_duration_seconds,
	index_album_art_pattern,
	ddns_host,
	ddns_username,
	ddns_password
) VALUES (
	0,
	randomblob(32),
	1800,
	"Folder.(jpeg|jpg|png)",
	"",
	"",
	""
);

CREATE TABLE mount_points (
	id INTEGER PRIMARY KEY NOT NULL,
	source TEXT NOT NULL,
	name TEXT NOT NULL,
	UNIQUE(name)
);

CREATE TABLE users (
	id INTEGER PRIMARY KEY NOT NULL,
	name TEXT NOT NULL,
	password_hash TEXT NOT NULL,
	admin INTEGER NOT NULL,
	lastfm_username TEXT,
	lastfm_session_key TEXT,
	web_theme_base TEXT,
	web_theme_accent TEXT,
	UNIQUE(name)
);

CREATE TABLE collection_index (
	id INTEGER PRIMARY KEY NOT NULL CHECK(id = 0),
	content BLOB
);

INSERT INTO collection_index (id, content) VALUES (0, NULL);
