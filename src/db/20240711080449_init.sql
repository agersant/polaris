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

CREATE TABLE directories (
	id INTEGER PRIMARY KEY NOT NULL,
	path TEXT NOT NULL,
	virtual_path TEXT NOT NULL,
	virtual_parent TEXT,
	UNIQUE(path) ON CONFLICT REPLACE
);

CREATE TABLE songs (
	id INTEGER PRIMARY KEY NOT NULL,
	path TEXT NOT NULL,
	virtual_path TEXT NOT NULL,
	virtual_parent TEXT NOT NULL,
	track_number INTEGER,
	disc_number INTEGER,
	title TEXT,
	artists TEXT,
	album_artists TEXT,
	year INTEGER,
	album TEXT,
	artwork TEXT,
	duration INTEGER,
	lyricists TEXT,
	composers TEXT,
	genres TEXT,
	labels TEXT,
	date_added INTEGER DEFAULT 0 NOT NULL,
	UNIQUE(path) ON CONFLICT REPLACE
);

CREATE TABLE collection_index (
	id INTEGER PRIMARY KEY NOT NULL CHECK(id = 0),
	content BLOB
);

INSERT INTO collection_index (id, content) VALUES (0, NULL);

CREATE TABLE playlists (
	id INTEGER PRIMARY KEY NOT NULL,
	owner INTEGER NOT NULL,
	name TEXT NOT NULL,
	FOREIGN KEY(owner) REFERENCES users(id) ON DELETE CASCADE,
	UNIQUE(owner, name) ON CONFLICT REPLACE
);

CREATE TABLE playlist_songs (
	id INTEGER PRIMARY KEY NOT NULL,
	playlist INTEGER NOT NULL,
	path TEXT NOT NULL,
	ordering INTEGER NOT NULL,
	FOREIGN KEY(playlist) REFERENCES playlists(id) ON DELETE CASCADE ON UPDATE CASCADE,
	UNIQUE(playlist, ordering) ON CONFLICT REPLACE
);
