use rocket::http::{Cookies, Status};
use rocket::request::{self, FromRequest, Request};
use rocket::{Outcome, State};
use rocket_contrib::json::Json;

use config::{self, Config};
use db::DB;
use errors;
use user;

const CURRENT_MAJOR_VERSION: i32 = 2;
const CURRENT_MINOR_VERSION: i32 = 2;

pub fn get_routes() -> Vec<rocket::Route> {
	routes![version, initial_setup, get_settings, put_settings]
}

struct Auth {
	username: String,
}

impl<'a, 'r> FromRequest<'a, 'r> for Auth {
	type Error = ();

	fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, ()> {
		let mut cookies = request.guard::<Cookies>().unwrap();
		match cookies.get_private("username") {
			Some(u) => Outcome::Success(Auth {
				username: u.to_string(),
			}),
			_ => Outcome::Failure((Status::Forbidden, ())),
		}
	}
}

struct AdminRights {}
impl<'a, 'r> FromRequest<'a, 'r> for AdminRights {
	type Error = ();

	fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, ()> {
		let db = request.guard::<State<DB>>()?;

		match user::count::<DB>(&db) {
			Err(_) => return Outcome::Failure((Status::InternalServerError, ())),
			Ok(0) => return Outcome::Success(AdminRights {}),
			_ => (),
		};

		let auth = request.guard::<Auth>()?;
		match user::is_admin::<DB>(&db, &auth.username) {
			Err(_) => Outcome::Failure((Status::InternalServerError, ())),
			Ok(true) => Outcome::Success(AdminRights {}),
			Ok(false) => Outcome::Failure((Status::Forbidden, ())),
		}
	}
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

#[derive(Serialize)]
struct InitialSetup {
	has_any_users: bool,
}

#[get("/initial_setup")]
fn initial_setup(db: State<DB>) -> Result<Json<InitialSetup>, errors::Error> {
	let initial_setup = InitialSetup {
		has_any_users: user::count::<DB>(&db)? > 0,
	};
	Ok(Json(initial_setup))
}

#[get("/settings")]
fn get_settings(db: State<DB>, _admin_rights: AdminRights) -> Result<Json<Config>, errors::Error> {
	let config = config::read::<DB>(&db)?;
	Ok(Json(config))
}

#[put("/settings", data = "<config>")]
fn put_settings(db: State<DB>, _admin_rights: AdminRights, config: Json<Config>) -> Result<(), errors::Error> {
	config::amend::<DB>(&db, &config)?;
	Ok(())
}
