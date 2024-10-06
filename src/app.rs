use std::fs;
use std::path::PathBuf;

use crate::db::DB;
use crate::paths::Paths;

pub mod config;
pub mod ddns;
pub mod formats;
pub mod index;
pub mod lastfm;
pub mod ndb;
pub mod peaks;
pub mod playlist;
pub mod scanner;
pub mod settings;
pub mod thumbnail;
pub mod user;
pub mod vfs;

#[cfg(test)]
pub mod test;

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error(transparent)]
	ThreadPoolBuilder(#[from] rayon::ThreadPoolBuildError),
	#[error(transparent)]
	ThreadJoining(#[from] tokio::task::JoinError),

	#[error("Filesystem error for `{0}`: `{1}`")]
	Io(PathBuf, std::io::Error),
	#[error(transparent)]
	Ape(#[from] ape::Error),
	#[error("ID3 error in `{0}`: `{1}`")]
	Id3(PathBuf, id3::Error),
	#[error("Metaflac error in `{0}`: `{1}`")]
	Metaflac(PathBuf, metaflac::Error),
	#[error("Mp4aMeta error in `{0}`: `{1}`")]
	Mp4aMeta(PathBuf, mp4ameta::Error),
	#[error(transparent)]
	Opus(#[from] opus_headers::ParseError),
	#[error(transparent)]
	Vorbis(#[from] lewton::VorbisError),
	#[error("Could not find a Vorbis comment within flac file")]
	VorbisCommentNotFoundInFlacFile,
	#[error("Could not read thumbnail image in `{0}`:\n\n{1}")]
	Image(PathBuf, image::error::ImageError),
	#[error("This file format is not supported: {0}")]
	UnsupportedFormat(&'static str),

	#[error("No tracks found in audio file: {0}")]
	MediaEmpty(PathBuf),
	#[error(transparent)]
	MediaDecodeError(symphonia::core::errors::Error),
	#[error(transparent)]
	MediaDecoderError(symphonia::core::errors::Error),
	#[error(transparent)]
	MediaPacketError(symphonia::core::errors::Error),
	#[error(transparent)]
	MediaProbeError(symphonia::core::errors::Error),

	#[error(transparent)]
	PeaksSerialization(bitcode::Error),
	#[error(transparent)]
	PeaksDeserialization(bitcode::Error),

	#[error(transparent)]
	Database(#[from] sqlx::Error),
	#[error("Could not initialize database connection pool")]
	ConnectionPoolBuild,
	#[error("Could not acquire database connection from pool")]
	ConnectionPool,
	#[error("Could not apply database migrations: {0}")]
	Migration(sqlx::migrate::MigrateError),

	#[error(transparent)]
	NativeDatabase(#[from] native_db::db_type::Error),
	#[error("Could not initialize database")]
	NativeDatabaseCreationError(native_db::db_type::Error),

	#[error("DDNS update query failed with HTTP status code `{0}`")]
	UpdateQueryFailed(u16),
	#[error("DDNS update query failed due to a transport error")]
	UpdateQueryTransport,

	#[error("Auth secret does not have the expected format")]
	AuthenticationSecretInvalid,
	#[error("Missing auth secret")]
	AuthenticationSecretNotFound,
	#[error("Missing settings")]
	MiscSettingsNotFound,
	#[error("Index album art pattern is not a valid regex")]
	IndexAlbumArtPatternInvalid,

	#[error(transparent)]
	Toml(#[from] toml::de::Error),
	#[error("Could not deserialize collection")]
	IndexDeserializationError,
	#[error("Could not serialize collection")]
	IndexSerializationError,

	#[error("The following virtual path could not be mapped to a real path: `{0}`")]
	CouldNotMapToRealPath(PathBuf),
	#[error("User not found")]
	UserNotFound,
	#[error("Directory not found: {0}")]
	DirectoryNotFound(PathBuf),
	#[error("Artist not found")]
	ArtistNotFound,
	#[error("Album not found")]
	AlbumNotFound,
	#[error("Genre not found")]
	GenreNotFound,
	#[error("Song not found")]
	SongNotFound,
	#[error("Invalid search query syntax")]
	SearchQueryParseError,
	#[error("Playlist not found")]
	PlaylistNotFound,
	#[error("No embedded artwork was found in `{0}`")]
	EmbeddedArtworkNotFound(PathBuf),

	#[error("Cannot use empty username")]
	EmptyUsername,
	#[error("Cannot use empty password")]
	EmptyPassword,
	#[error("Username does not exist")]
	IncorrectUsername,
	#[error("Password does not match username")]
	IncorrectPassword,
	#[error("Invalid auth token")]
	InvalidAuthToken,
	#[error("Incorrect authorization scope")]
	IncorrectAuthorizationScope,
	#[error("Last.fm session key is missing")]
	MissingLastFMSessionKey,
	#[error("Failed to hash password")]
	PasswordHashing,
	#[error("Failed to encode authorization token")]
	AuthorizationTokenEncoding,
	#[error("Failed to encode Branca token")]
	BrancaTokenEncoding,

	#[error("Failed to authenticate with last.fm")]
	ScrobblerAuthentication(rustfm_scrobble::ScrobblerError),
	#[error("Failed to emit last.fm scrobble")]
	Scrobble(rustfm_scrobble::ScrobblerError),
	#[error("Failed to emit last.fm now playing update")]
	NowPlaying(rustfm_scrobble::ScrobblerError),
}

#[derive(Clone)]
pub struct App {
	pub port: u16,
	pub web_dir_path: PathBuf,
	pub swagger_dir_path: PathBuf,
	pub scanner: scanner::Scanner,
	pub index_manager: index::Manager,
	pub config_manager: config::Manager,
	pub ddns_manager: ddns::Manager,
	pub lastfm_manager: lastfm::Manager,
	pub peaks_manager: peaks::Manager,
	pub playlist_manager: playlist::Manager,
	pub settings_manager: settings::Manager,
	pub thumbnail_manager: thumbnail::Manager,
	pub user_manager: user::Manager,
	pub vfs_manager: vfs::Manager,
}

impl App {
	pub async fn new(port: u16, paths: Paths) -> Result<Self, Error> {
		let db = DB::new(&paths.db_file_path).await?;

		fs::create_dir_all(&paths.data_dir_path)
			.map_err(|e| Error::Io(paths.data_dir_path.clone(), e))?;

		fs::create_dir_all(&paths.web_dir_path)
			.map_err(|e| Error::Io(paths.web_dir_path.clone(), e))?;

		fs::create_dir_all(&paths.swagger_dir_path)
			.map_err(|e| Error::Io(paths.swagger_dir_path.clone(), e))?;

		let peaks_dir_path = paths.cache_dir_path.join("peaks");
		fs::create_dir_all(&peaks_dir_path).map_err(|e| Error::Io(peaks_dir_path.clone(), e))?;

		let thumbnails_dir_path = paths.cache_dir_path.join("thumbnails");
		fs::create_dir_all(&thumbnails_dir_path)
			.map_err(|e| Error::Io(thumbnails_dir_path.clone(), e))?;

		let ndb_manager = ndb::Manager::new(&paths.data_dir_path)?;
		let vfs_manager = vfs::Manager::new(db.clone());
		let settings_manager = settings::Manager::new(db.clone());
		let auth_secret = settings_manager.get_auth_secret().await?;
		let ddns_manager = ddns::Manager::new(db.clone());
		let user_manager = user::Manager::new(db.clone(), auth_secret);
		let index_manager = index::Manager::new(db.clone()).await;
		let scanner = scanner::Scanner::new(
			index_manager.clone(),
			settings_manager.clone(),
			vfs_manager.clone(),
		)
		.await?;
		let config_manager = config::Manager::new(
			settings_manager.clone(),
			user_manager.clone(),
			vfs_manager.clone(),
			ddns_manager.clone(),
		);
		let peaks_manager = peaks::Manager::new(peaks_dir_path);
		let playlist_manager = playlist::Manager::new(ndb_manager);
		let thumbnail_manager = thumbnail::Manager::new(thumbnails_dir_path);
		let lastfm_manager = lastfm::Manager::new(index_manager.clone(), user_manager.clone());

		if let Some(config_path) = paths.config_file_path {
			let config = config::Config::from_path(&config_path)?;
			config_manager.apply(&config).await?;
		}

		Ok(Self {
			port,
			web_dir_path: paths.web_dir_path,
			swagger_dir_path: paths.swagger_dir_path,
			scanner,
			index_manager,
			config_manager,
			ddns_manager,
			lastfm_manager,
			peaks_manager,
			playlist_manager,
			settings_manager,
			thumbnail_manager,
			user_manager,
			vfs_manager,
		})
	}
}
