use axum::Router;
use tower_http::services::ServeDir;

use crate::app::App;

mod api;

#[cfg(test)]
pub mod test;

pub fn make_router(app: App) -> Router {
	Router::new()
		.nest_service("/swagger", ServeDir::new(app.swagger_dir_path))
		.nest_service("/", ServeDir::new(app.web_dir_path))
	// move |cfg: &mut ServiceConfig| {
	// 	cfg.app_data(web::Data::new(app.index))
	// 		.app_data(web::Data::new(app.config_manager))
	// 		.app_data(web::Data::new(app.ddns_manager))
	// 		.app_data(web::Data::new(app.lastfm_manager))
	// 		.app_data(web::Data::new(app.playlist_manager))
	// 		.app_data(web::Data::new(app.settings_manager))
	// 		.app_data(web::Data::new(app.thumbnail_manager))
	// 		.app_data(web::Data::new(app.user_manager))
	// 		.app_data(web::Data::new(app.vfs_manager))
	// 		.service(
	// 			web::scope("/api")
	// 				.configure(api::make_config())
	// 				.wrap(NormalizePath::trim()),
	// 		)
	// }
}

pub async fn launch(app: App) -> Result<(), std::io::Error> {
	let port = app.port;
	let router = make_router(app);

	let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
	axum::serve(listener, router).await?;

	Ok(())
}
