use axum::Router;

use crate::app::App;

mod api;

#[cfg(test)]
pub mod test;

pub fn make_router(app: App) -> Router {
	Router::new()
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
	// 		.service(
	// 			actix_files::Files::new("/swagger", app.swagger_dir_path)
	// 				.redirect_to_slash_directory()
	// 				.index_file("index.html"),
	// 		)
	// 		.service(
	// 			actix_files::Files::new("/", app.web_dir_path)
	// 				.redirect_to_slash_directory()
	// 				.index_file("index.html"),
	// 		);
	// }
}

pub async fn launch(app: App) -> Result<(), std::io::Error> {
	let port = app.port;
	let router = make_router(app);

	let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
	axum::serve(listener, router).await?;

	Ok(())
}
