CREATE TEMPORARY TABLE songs_backup(id, path, parent, track_number, disc_number, title, artist, album_artist, year, album, artwork, duration);
INSERT INTO songs_backup SELECT id, path, parent, track_number, disc_number, title, artist, album_artist, year, album, artwork, duration FROM songs;
DROP TABLE songs;
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
  duration INTEGER,
	UNIQUE(path) ON CONFLICT REPLACE
);
INSERT INTO songs SELECT * FROM songs_backup;
DROP TABLE songs_backup;
