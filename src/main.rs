extern crate iron;
extern crate router;
extern crate mount;
extern crate staticfile;

use iron::prelude::*;
use iron::status;
use router::Router;
use mount::Mount;
use staticfile::Static;


fn main() {
    let mut mount = Mount::new();
    mount.mount( "/static/", Static::new("samplemusic/") );

    let mut router = Router::new();
    router.get("/static/*", mount );
    router.get("/api/*", |_: &mut Request| {
        Ok(Response::with((status::Ok, "API")))
    } );
    router.get("/web/*", |_: &mut Request| {
        Ok(Response::with((status::Ok, "Web")))
    } );
    router.get("/", |_: &mut Request| {
        Ok(Response::with((status::Ok, "Home")))
    } );

    Iron::new(router).http("localhost:3000").unwrap();
}