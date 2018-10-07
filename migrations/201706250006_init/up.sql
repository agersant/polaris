CREATE TABLE directories (
	id INTEGER PRIMARY KEY NOT NULL,
	path TEXT NOT NULL,
	parent TEXT,
	artist TEXT,
	year INTEGER,
	album TEXT,
	artwork TEXT,
	UNIQUE(path) ON CONFLICT REPLACE
);

CREATE TABLE songs (
	id INTEGER PRIMARY KEY NOT NULL,
	path TEXT NOT NULL,
	parent TEXT NOT NULL,
	track_number INTEGER,
	disc_number INTEGER,
	title TEXT,
	artist TEXT,
	album_artist TEXT,
	year INTEGER,
	album TEXT,
	artwork TEXT, 
	UNIQUE(path) ON CONFLICT REPLACE
);