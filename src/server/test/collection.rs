use http::StatusCode;

use crate::{
	server::{
		dto,
		test::{
			add_trailing_slash,
			protocol::{self, V7, V8},
			ServiceType, TestService,
		},
	},
	test_name,
};

#[tokio::test]
async fn random_requires_auth() {
	let mut service = ServiceType::new(&test_name!()).await;
	let request = protocol::random::<V8>();
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn random_golden_path() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let request = protocol::random::<V8>();
	let response = service
		.fetch_json::<_, Vec<dto::AlbumHeader>>(&request)
		.await;
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 3);
}

#[tokio::test]
async fn random_with_trailing_slash() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let mut request = protocol::random::<V8>();
	add_trailing_slash(&mut request);
	let response = service
		.fetch_json::<_, Vec<dto::AlbumHeader>>(&request)
		.await;
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 3);
}

#[tokio::test]
async fn random_golden_path_api_v7() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let request = protocol::random::<V7>();
	let response = service
		.fetch_json::<_, Vec<dto::v7::Directory>>(&request)
		.await;
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 3);
	assert!(entries[0].path.starts_with("collection/"));
}

#[tokio::test]
async fn recent_requires_auth() {
	let mut service = ServiceType::new(&test_name!()).await;
	let request = protocol::recent::<V8>();
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn recent_golden_path() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let request = protocol::recent::<V8>();
	let response = service
		.fetch_json::<_, Vec<dto::AlbumHeader>>(&request)
		.await;
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 3);
}

#[tokio::test]
async fn recent_with_trailing_slash() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let mut request = protocol::recent::<V8>();
	add_trailing_slash(&mut request);
	let response = service
		.fetch_json::<_, Vec<dto::AlbumHeader>>(&request)
		.await;
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 3);
}

#[tokio::test]
async fn recent_golden_path_api_v7() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let request = protocol::recent::<V7>();
	let response = service
		.fetch_json::<_, Vec<dto::v7::Directory>>(&request)
		.await;
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 3);
	assert!(entries[0].path.starts_with("collection/"));
}

#[tokio::test]
async fn genres_requires_auth() {
	let mut service = ServiceType::new(&test_name!()).await;
	let request = protocol::genres::<V8>();
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn genres_golden_path() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let request = protocol::genres::<V8>();
	let response = service
		.fetch_json::<_, Vec<dto::GenreHeader>>(&request)
		.await;
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 4);
}

#[tokio::test]
async fn genre_requires_auth() {
	let mut service = ServiceType::new(&test_name!()).await;
	let request = protocol::genre::<V8>("Metal");
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn genre_golden_path() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let request = protocol::genre::<V8>("Metal");
	let response = service.fetch_json::<_, dto::Genre>(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
	let genre = response.body();
	assert_eq!(genre.header.name, "Metal");
}

#[tokio::test]
async fn genre_albums_requires_auth() {
	let mut service = ServiceType::new(&test_name!()).await;
	let request = protocol::genre_albums::<V8>("Metal");
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn genre_albums_golden_path() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let request = protocol::genre_albums::<V8>("Metal");
	let response = service
		.fetch_json::<_, Vec<dto::AlbumHeader>>(&request)
		.await;
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 1);
}

#[tokio::test]
async fn genre_artists_requires_auth() {
	let mut service = ServiceType::new(&test_name!()).await;
	let request = protocol::genre_artists::<V8>("Metal");
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn genre_artists_golden_path() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let request = protocol::genre_artists::<V8>("Metal");
	let response = service
		.fetch_json::<_, Vec<dto::ArtistHeader>>(&request)
		.await;
	assert_eq!(response.status(), StatusCode::OK);
	let entries = response.body();
	assert_eq!(entries.len(), 1);
}

#[tokio::test]
async fn genre_songs_requires_auth() {
	let mut service = ServiceType::new(&test_name!()).await;
	let request = protocol::genre_songs::<V8>("Metal");
	let response = service.fetch(&request).await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn genre_songs_golden_path() {
	let mut service = ServiceType::new(&test_name!()).await;
	service.complete_initial_setup().await;
	service.login_admin().await;
	service.index().await;
	service.login().await;

	let request = protocol::genre_songs::<V8>("Metal");
	let response = service.fetch_json::<_, dto::SongList>(&request).await;
	assert_eq!(response.status(), StatusCode::OK);
	let song_list = response.body();
	assert_eq!(song_list.paths.len(), 5);
}
