use rocket::http::uri::Origin;
use rocket::response::NamedFile;
use rocket::response::Redirect;
use rocket::State;
use std::path::PathBuf;
use std::sync::Arc;

use crate::server::StaticDirs;

pub fn get_routes() -> Vec<rocket::Route> {
	routes![
		index,
		files,
	]
}

#[get("/", rank = 9)]
fn index(origin: &Origin) -> Redirect {
	let mut new_path = origin.path().to_owned();
	if !new_path.ends_with("/") {
		new_path.push_str("/");
	}
	new_path.push_str("index.html");
	let redirect = Redirect::permanent(new_path);
	return redirect;
}

#[get("/<file..>", rank = 9)]
fn files(static_dirs: State<Arc<StaticDirs>>, file: PathBuf) -> Option<NamedFile> {
	let path = static_dirs.swagger_dir_path.clone().join(file.clone());
    NamedFile::open(path).ok()
}

#[test]
fn test_index_redirect() {
	use rocket::http::Status;
	use crate::test::get_test_environment;

	let env = get_test_environment("swagger_index_redirect.sqlite");
	let client = &env.client;
	let response = client.get("/swagger").dispatch();
	assert_eq!(response.status(), Status::PermanentRedirect);
	assert_eq!(response.headers().get_one("Location"), Some("/swagger/index.html"));
}

#[test]
fn test_index() {
	use rocket::http::Status;
	use crate::test::get_test_environment;

	let env = get_test_environment("swagger_index.sqlite");
	let client = &env.client;
	let response = client.get("/swagger/index.html").dispatch();
	assert_eq!(response.status(), Status::Ok);
}
