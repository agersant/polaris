use app_dirs::{AppDataType, data_root};
use std::path::{Path, PathBuf};
use std::fs;

use error::PError;

pub fn get_config_root() -> Result<PathBuf, PError> {
    if let Ok(mut root) = data_root(AppDataType::SharedConfig){
        root.push("Polaris");
        return match fs::create_dir_all(&root) {
            Ok(()) => Ok(root),
            Err(_) => Err(PError::CacheDirectoryError),
        }
    } 
    Err(PError::ConfigDirectoryError)
}

pub fn get_cache_root() -> Result<PathBuf, PError> {
    if let Ok(mut root) = data_root(AppDataType::SharedData){
        root.push("Polaris");
        return match fs::create_dir_all(&root) {
            Ok(()) => Ok(root),
            Err(_) => Err(PError::CacheDirectoryError),
        }
    } 
    Err(PError::CacheDirectoryError)
}

#[derive(Debug, PartialEq)]
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

#[test]
fn test_get_audio_format() {
    assert_eq!(get_audio_format(Path::new("animals/🐷/my🐖file.jpg")), None);
    assert_eq!(get_audio_format(Path::new("animals/🐷/my🐖file.flac")), Some(AudioFormat::FLAC));
}

pub fn is_song(path: &Path) -> bool {
    get_audio_format(path).is_some()
}

#[test]
fn test_is_song() {
    assert!(is_song(Path::new("animals/🐷/my🐖file.mp3")));
    assert!(!is_song(Path::new("animals/🐷/my🐖file.jpg")));
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

#[test]
fn test_is_image() {
    assert!(!is_image(Path::new("animals/🐷/my🐖file.mp3")));
    assert!(is_image(Path::new("animals/🐷/my🐖file.jpg")));
}
