-- songs
CREATE TEMPORARY TABLE songs_backup(id, path, parent, track_number, disc_number, title, year, album, artwork, duration, lyricist, composer, genre, label);
INSERT INTO songs_backup SELECT * FROM songs;
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
	lyricist TEXT,
	composer TEXT,
	genre TEXT,
	label TEXT,
	UNIQUE(path)
);
INSERT INTO songs
    SELECT s.id, s.path, s.parent, s.track_number, s.disc_number, s.title, s.year, s.album, s.artwork, s.duration, s.lyricist, s.composer, s.genre, s.label, a.name AS artist, aa.name AS album_artist
        FROM songs_backup s
        INNER JOIN song_artists sa ON sa.song = s.id
        INNER JOIN artists a ON a.id = sa.artist
        INNER JOIN song_album_artists saa ON saa.song = s.id
        INNER JOIN artists aa ON aa.id = saa.artist
        GROUP BY s.id;
DROP TABLE songs_backup;
DROP TABLE song_artists;
DROP TABLE song_album_artists;

-- directories
CREATE TEMPORARY TABLE directories_backup(id, path, parent, year, album, artwork, date_added);
INSERT INTO directories_backup SELECT * FROM directories;
DROP TABLE directories;
CREATE TABLE directories (
	id INTEGER PRIMARY KEY NOT NULL,
	path TEXT NOT NULL,
	parent TEXT,
    artist TEXT,
	year TEXT,
	album TEXT,
	artwork TEXT,
	date_added INTEGER NOT NULL,
	UNIQUE(path)
);
INSERT INTO directories
    SELECT d.id, d.path, d.parent, d.year, d.album, d.artwork, d.date_added, a.name AS artist
        FROM directories_backup d
        INNER JOIN directory_artists da ON da.directory = d.id
        INNER JOIN artists a ON a.id = da.artist
        GROUP BY d.id;
DROP TABLE directories_backup;
DROP TABLE directory_artists;

-- artists
DROP TABLE artists;
