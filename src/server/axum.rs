use crate::app::{self, App};
use crate::server::doc;
use axum::{extract::FromRef, Router, ServiceExt};
use tower::Layer;
use tower_http::{
	compression::CompressionLayer,
	normalize_path::{NormalizePath, NormalizePathLayer},
	services::ServeDir,
};
use utoipa_axum::router::OpenApiRouter;
use utoipa_scalar::{Scalar, Servable};

mod api;
mod auth;
mod error;
mod logger;
mod version;

#[cfg(test)]
pub mod test;

pub fn make_router(app: App) -> NormalizePath<Router> {
	let static_files = Router::new()
		.fallback_service(ServeDir::new(&app.web_dir_path))
		.layer(CompressionLayer::new());

	let (open_api_router, open_api) = OpenApiRouter::with_openapi(doc::open_api())
		.nest("/api", api::router())
		.split_for_parts();

	let router = open_api_router
		.with_state(app.clone())
		.merge(Scalar::with_url("/api-docs", open_api))
		.fallback_service(static_files)
		.layer(logger::LogLayer::new());

	NormalizePathLayer::trim_trailing_slash().layer(router)
}

pub async fn launch(app: App) -> Result<(), std::io::Error> {
	let port = app.port;
	let router = make_router(app);
	let make_service = ServiceExt::<axum::extract::Request>::into_make_service(router);
	let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
	tokio::spawn(async {
		axum::serve(listener, make_service).await.unwrap();
	});
	Ok(())
}

impl FromRef<App> for app::index::Manager {
	fn from_ref(app: &App) -> Self {
		app.index_manager.clone()
	}
}

impl FromRef<App> for app::scanner::Scanner {
	fn from_ref(app: &App) -> Self {
		app.scanner.clone()
	}
}

impl FromRef<App> for app::config::Manager {
	fn from_ref(app: &App) -> Self {
		app.config_manager.clone()
	}
}

impl FromRef<App> for app::ddns::Manager {
	fn from_ref(app: &App) -> Self {
		app.ddns_manager.clone()
	}
}

impl FromRef<App> for app::peaks::Manager {
	fn from_ref(app: &App) -> Self {
		app.peaks_manager.clone()
	}
}

impl FromRef<App> for app::playlist::Manager {
	fn from_ref(app: &App) -> Self {
		app.playlist_manager.clone()
	}
}

impl FromRef<App> for app::thumbnail::Manager {
	fn from_ref(app: &App) -> Self {
		app.thumbnail_manager.clone()
	}
}
