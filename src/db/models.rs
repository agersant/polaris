use rand;
use ring::{digest, pbkdf2};

use db::schema::*;

// Collection content
#[derive(Debug, Queryable, Serialize)]
pub struct Song {
	#[serde(skip_serializing)]
	id: i32,
	pub path: String,
	#[serde(skip_serializing)]
	pub parent: String,
	pub track_number: Option<i32>,
	pub disc_number: Option<i32>,
	pub title: Option<String>,
	pub artist: Option<String>,
	pub album_artist: Option<String>,
	pub year: Option<i32>,
	pub album: Option<String>,
	pub artwork: Option<String>,
}

#[derive(Debug, Queryable, Serialize)]
pub struct Directory {
	#[serde(skip_serializing)]
	id: i32,
	pub path: String,
	#[serde(skip_serializing)]
	pub parent: Option<String>,
	pub artist: Option<String>,
	pub year: Option<i32>,
	pub album: Option<String>,
	pub artwork: Option<String>,
	pub date_added: i32,
}

#[derive(Debug, Serialize)]
pub enum CollectionFile {
	Directory(Directory),
	Song(Song),
}


// User
#[derive(Debug, Queryable)]
pub struct User {
	id: i32,
	pub name: String,
	pub password_salt: Vec<u8>,
	pub password_hash: Vec<u8>,
}

impl User {
	pub fn verify_password(&self, attempted_password: &str) -> bool {
		pbkdf2::verify(DIGEST_ALG,
		               HASH_ITERATIONS,
		               &self.password_salt,
		               attempted_password.as_bytes(),
		               &self.password_hash)
				.is_ok()
	}
}

#[derive(Debug, Insertable)]
#[table_name="users"]
pub struct NewUser {
	pub name: String,
	pub password_salt: Vec<u8>,
	pub password_hash: Vec<u8>,
}

static DIGEST_ALG: &'static pbkdf2::PRF = &pbkdf2::HMAC_SHA256;
const CREDENTIAL_LEN: usize = digest::SHA256_OUTPUT_LEN;
const HASH_ITERATIONS: u32 = 10000;
type PasswordHash = [u8; CREDENTIAL_LEN];

impl NewUser {
	pub fn new(name: &str, password: &str) -> NewUser {
		let salt = rand::random::<[u8; 16]>().to_vec();
		let hash = NewUser::hash_password(&salt, password);
		NewUser {
			name: name.to_owned(),
			password_salt: salt,
			password_hash: hash,
		}
	}

	pub fn hash_password(salt: &Vec<u8>, password: &str) -> Vec<u8> {
		let mut hash: PasswordHash = [0; CREDENTIAL_LEN];
		pbkdf2::derive(DIGEST_ALG,
		               HASH_ITERATIONS,
		               salt,
		               password.as_bytes(),
		               &mut hash);
		hash.to_vec()
	}
}


// VFS
#[derive(Debug, Deserialize, Insertable, Queryable)]
#[table_name="mount_points"]
pub struct MountPoint {
	pub source: String,
	pub name: String,
}


// Misc Settings
#[derive(Debug, Queryable)]
pub struct MiscSettings {
	id: i32,
	pub auth_secret: String,
	pub index_sleep_duration_seconds: i32,
	pub index_album_art_pattern: String,
}
