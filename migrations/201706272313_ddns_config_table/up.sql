CREATE TABLE ddns_config (
	id INTEGER PRIMARY KEY NOT NULL CHECK(id = 0),
	host TEXT NOT NULL,
	username TEXT NOT NULL,
	password TEXT NOT NULL
);

INSERT INTO ddns_config (id, host, username, password) VALUES (0, "", "", "");