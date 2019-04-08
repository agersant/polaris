use rocket::http::Status;
use rocket::response::NamedFile;
use rocket::State;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use crate::server::StaticDirs;
use crate::test::get_test_environment;

pub fn get_routes() -> Vec<rocket::Route> {
	routes![
		index,
		files,
	]
}

#[get("/", rank = 10)]
fn index(static_dirs: State<Arc<StaticDirs>>) -> io::Result<NamedFile> {
	let mut path = static_dirs.web_dir_path.clone();
	path.push("index.html");
    NamedFile::open(path)
}

#[get("/<file..>", rank = 10)]
fn files(static_dirs: State<Arc<StaticDirs>>, file: PathBuf) -> Option<NamedFile> {
	let path = static_dirs.web_dir_path.clone().join(file.clone());
    NamedFile::open(path).ok()
}

#[test]
fn test_index() {
	let env = get_test_environment("web_index.sqlite");
	let client = &env.client;
	let response = client.get("/").dispatch();
	assert_eq!(response.status(), Status::Ok);
}
