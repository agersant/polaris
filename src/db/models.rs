// Misc Settings
#[derive(Debug, Queryable)]
pub struct MiscSettings {
	id: i32,
	pub auth_secret: String,
	pub index_sleep_duration_seconds: i32,
	pub index_album_art_pattern: String,
}
