CREATE TEMPORARY TABLE songs_backup(path, parent, track_number, disc_number, title, year, album, artwork, duration, lyricist, composer, genre, label);
INSERT INTO songs_backup SELECT id, auth_secret, index_sleep_duration_seconds, index_album_art_pattern FROM misc_settings;

CREATE TABLE artists (
	id INTEGER PRIMARY KEY NOT NULL,
	name TEXT NOT NULL,
	UNIQUE(name) ON CONFLICT REPLACE
);

CREATE TABLE song_artists (
	song INTEGER NOT NULL,
	artist INTEGER NOT NULL,
    PRIMARY KEY (song, artist),
	FOREIGN KEY(song) REFERENCES songs(id) ON DELETE CASCADE ON UPDATE CASCADE,
	FOREIGN KEY(artist) REFERENCES artists(id) ON DELETE CASCADE ON UPDATE CASCADE,
	UNIQUE(song, artist) ON CONFLICT REPLACE
);

CREATE TABLE song_album_artists (
	song INTEGER NOT NULL,
	artist INTEGER NOT NULL,
    PRIMARY KEY (song, artist),
	FOREIGN KEY(song) REFERENCES songs(id) ON DELETE CASCADE ON UPDATE CASCADE,
	FOREIGN KEY(artist) REFERENCES artists(id) ON DELETE CASCADE ON UPDATE CASCADE,
	UNIQUE(song, artist) ON CONFLICT REPLACE
);

CREATE TABLE directory_artists (
	directory INTEGER NOT NULL,
	artist INTEGER NOT NULL,
    PRIMARY KEY (directory, artist),
	FOREIGN KEY(directory) REFERENCES directories(id) ON DELETE CASCADE ON UPDATE CASCADE,
	FOREIGN KEY(artist) REFERENCES artists(id) ON DELETE CASCADE ON UPDATE CASCADE,
	UNIQUE(directory, artist) ON CONFLICT REPLACE
);
