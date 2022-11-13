table! {
	ddns_config (id) {
		id -> Integer,
		host -> Text,
		username -> Text,
		password -> Text,
	}
}

table! {
	directories (id) {
		id -> Integer,
		path -> Text,
		parent -> Nullable<Text>,
		year -> Nullable<Integer>,
		album -> Nullable<Text>,
		artwork -> Nullable<Text>,
		date_added -> Integer,
	}
}

table! {
    directory_artists(directory, artist) {
        directory -> Integer,
        artist -> Integer,
    }
}

table! {
	misc_settings (id) {
		id -> Integer,
		auth_secret -> Binary,
		index_sleep_duration_seconds -> Integer,
		index_album_art_pattern -> Text,
	}
}

table! {
	mount_points (id) {
		id -> Integer,
		source -> Text,
		name -> Text,
	}
}

table! {
	playlist_songs (id) {
		id -> Integer,
		playlist -> Integer,
		path -> Text,
		ordering -> Integer,
	}
}

table! {
	playlists (id) {
		id -> Integer,
		owner -> Integer,
		name -> Text,
	}
}

table! {
	songs (id) {
		id -> Integer,
		path -> Text,
		parent -> Text,
		track_number -> Nullable<Integer>,
		disc_number -> Nullable<Integer>,
		title -> Nullable<Text>,
		year -> Nullable<Integer>,
		album -> Nullable<Text>,
		artwork -> Nullable<Text>,
		duration -> Nullable<Integer>,
		lyricist -> Nullable<Text>,
		composer -> Nullable<Text>,
		genre -> Nullable<Text>,
		label -> Nullable<Text>,
	}
}

table! {
    song_artists(song, artist) {
        song -> Integer,
        artist -> Integer,
    }
}

table! {
    song_album_artists(song, artist) {
        song -> Integer,
        artist -> Integer,
    }
}

table! {
    artists(id) {
        id -> Integer,
        name -> Text,
    }
}

table! {
	users (id) {
		id -> Integer,
		name -> Text,
		password_hash -> Text,
		admin -> Integer,
		lastfm_username -> Nullable<Text>,
		lastfm_session_key -> Nullable<Text>,
		web_theme_base -> Nullable<Text>,
		web_theme_accent -> Nullable<Text>,
	}
}

joinable!(song_artists -> songs (song));
joinable!(song_artists -> artists (artist));
joinable!(song_album_artists -> songs (song));
joinable!(song_album_artists -> artists (artist));
joinable!(directory_artists -> artists (artist));
joinable!(playlist_songs -> playlists (playlist));
joinable!(playlists -> users (owner));

allow_tables_to_appear_in_same_query!(
    artists,
	ddns_config,
	directories,
    directory_artists,
	misc_settings,
	mount_points,
	playlist_songs,
	playlists,
	songs,
    song_artists,
    song_album_artists,
	users,
);
