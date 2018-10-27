use rocket_contrib::json::Json;

const CURRENT_MAJOR_VERSION: i32 = 2;
const CURRENT_MINOR_VERSION: i32 = 2;

pub fn get_routes() -> Vec<rocket::Route> {
	routes![version]
}

#[derive(Serialize)]
struct Version {
	major: i32,
	minor: i32,
}

#[get("/version")]
fn version() -> Json<Version> {
	let current_version = Version {
		major: CURRENT_MAJOR_VERSION,
		minor: CURRENT_MINOR_VERSION,
	};
	Json(current_version)
}
