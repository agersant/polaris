use std::path::Path;

pub enum AudioFormat {
	FLAC,
	MP3,
	MP4,
	MPC,
	OGG,
}

pub fn get_audio_format(path: &Path) -> Option<AudioFormat> {
	let extension = match path.extension() {
		Some(e) => e,
		_ => return None,
	};
	let extension = match extension.to_str() {
		Some(e) => e,
		_ => return None,
	};
	match extension.to_lowercase().as_str() {
		"flac" => Some(AudioFormat::FLAC),
		"mp3" => Some(AudioFormat::MP3),
		"m4a" => Some(AudioFormat::MP4),
		"mpc" => Some(AudioFormat::MPC),
		"ogg" => Some(AudioFormat::OGG),
		_ => None,
	}
}

pub fn is_song(path: &Path) -> bool {
	get_audio_format(path).is_some()
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