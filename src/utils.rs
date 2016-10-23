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
	match extension {
		"mp3" => return true,
		"ogg" => return true,
		"m4a" => return true,
		"flac" => return true,
		_ => return false,
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
	match extension {
		"png" => return true,
		"gif" => return true,
		"jpg" => return true,
		"bmp" => return true,
		_ => return false,
	}
}