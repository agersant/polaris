CREATE TABLE artists (
	id INTEGER PRIMARY KEY NOT NULL,
	name TEXT NOT NULL,
	UNIQUE(name)
);

-- songs
CREATE TEMPORARY TABLE songs_backup(id, path, parent, track_number, disc_number, title, artist, album_artist, year, album, artwork, duration, lyricist, composer, genre, label);
INSERT INTO songs_backup SELECT * FROM songs;
DROP TABLE songs;
CREATE TABLE songs (
	id INTEGER PRIMARY KEY NOT NULL,
	path TEXT NOT NULL,
	parent TEXT NOT NULL,
	track_number INTEGER,
	disc_number INTEGER,
	title TEXT,
	year INTEGER,
	album TEXT,
	artwork TEXT,
	duration INTEGER,
	lyricist TEXT,
	composer TEXT,
	genre TEXT,
	label TEXT,
	UNIQUE(path)
);
INSERT INTO songs SELECT id, path, parent, track_number, disc_number, title, artist, album_artist, year, album, artwork, duration, lyricist, composer, genre, label FROM songs_backup;

CREATE TABLE song_artists (
	song INTEGER NOT NULL,
	artist INTEGER NOT NULL,
    PRIMARY KEY (song, artist),
	FOREIGN KEY(song) REFERENCES songs(id) ON DELETE CASCADE ON UPDATE CASCADE,
	FOREIGN KEY(artist) REFERENCES artists(id) ON DELETE CASCADE ON UPDATE CASCADE,
	UNIQUE(song, artist)
);

CREATE TABLE song_album_artists (
	song INTEGER NOT NULL,
	artist INTEGER NOT NULL,
    PRIMARY KEY (song, artist),
	FOREIGN KEY(song) REFERENCES songs(id) ON DELETE CASCADE ON UPDATE CASCADE,
	FOREIGN KEY(artist) REFERENCES artists(id) ON DELETE CASCADE ON UPDATE CASCADE,
	UNIQUE(song, artist)
);

INSERT INTO song_artists
    SELECT 
    sb.id AS song,
    (INSERT INTO artists sb.artist AS name
        ON CONFLICT DO UPDATE SET artists.name = excluded.name
        RETURNING ID
    ) AS artist
    FROM songs_backup sb;

INSERT INTO song_album_artists
    SELECT 
    sb.id AS song,
    (INSERT INTO artists sb.album_artist AS name
        ON CONFLICT DO UPDATE SET artists.name = excluded.name
        RETURNING ID
    ) AS artist
    FROM songs_backup sb;

DROP TABLE songs_backup;

-- directories
CREATE TEMPORARY TABLE directories_backup(id, path, parent, artist, year, album, artwork, date_added);
INSERT INTO directories_backup SELECT * FROM directories;
DROP TABLE directories;
CREATE TABLE directories (
	id INTEGER PRIMARY KEY NOT NULL,
	path TEXT NOT NULL,
	parent TEXT,
	year TEXT,
	album TEXT,
	artwork TEXT,
	date_added INTEGER NOT NULL,
	UNIQUE(path)
);
INSERT INTO directories SELECT id, path, parent, year, album, artwork, date_added;

CREATE TABLE directory_artists (
	directory INTEGER NOT NULL,
	artist INTEGER NOT NULL,
    PRIMARY KEY (directory, artist),
	FOREIGN KEY(directory) REFERENCES directories(id) ON DELETE CASCADE ON UPDATE CASCADE,
	FOREIGN KEY(artist) REFERENCES artists(id) ON DELETE CASCADE ON UPDATE CASCADE,
	UNIQUE(directory, artist)
);

INSERT INTO directory_artists
    SELECT 
    sb.id AS directory,
    (INSERT INTO artists db.artist AS name
        ON CONFLICT DO UPDATE SET artists.name = excluded.name
        RETURNING ID
    ) AS artist
    FROM directory_backup db;
DROP TABLE directories_backup;
