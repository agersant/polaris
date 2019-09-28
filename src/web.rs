#[test]
fn test_index() {
	use crate::test::get_test_environment;
	use rocket::http::Status;

	let env = get_test_environment("web_index.sqlite");
	let client = &env.client;
	let response = client.get("/").dispatch();
	assert_eq!(response.status(), Status::Ok);
}
