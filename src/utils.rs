use std::path::Path;

pub fn is_song(path: &Path) -> bool {
	let extension = match path.extension() {
		Some(e) => e,
		_ => return false,
	};
	let extension = match extension.to_str() {
		Some(e) => e,
		_ => return false,
	};
	match extension.to_lowercase().as_str() {
		"mp3" => true,
		"ogg" => true,
		"m4a" => true,
		"flac" => true,
		_ => false,
	}
}

pub fn is_image(path: &Path) -> bool {
	let extension = match path.extension() {
		Some(e) => e,
		_ => return false,
	};
	let extension = match extension.to_str() {
		Some(e) => e,
		_ => return false,
	};
	match extension.to_lowercase().as_str() {
		"png" => true,
		"gif" => true,
		"jpg" => true,
		"jpeg" => true,
		"bmp" => true,
		_ => false,
	}
}