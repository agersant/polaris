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
		artist -> Nullable<Text>,
		year -> Nullable<Integer>,
		album -> Nullable<Text>,
		artwork -> Nullable<Text>,
		date_added -> Integer,
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
		artist -> Nullable<Text>,
		album_artist -> Nullable<Text>,
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

joinable!(playlist_songs -> playlists (playlist));
joinable!(playlists -> users (owner));

allow_tables_to_appear_in_same_query!(
	ddns_config,
	directories,
	misc_settings,
	mount_points,
	playlist_songs,
	playlists,
	songs,
	users,
);
