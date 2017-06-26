CREATE TEMPORARY TABLE directories_backup(id, path, parent, artist, year, album, artwork);
INSERT INTO directories_backup SELECT id, path, parent, artist, year, album, artwork FROM directories;
DROP TABLE directories;
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
INSERT INTO directories SELECT * FROM directories_backup;
DROP TABLE directories_backup;