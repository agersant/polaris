use axum::{extract::FromRef, Router};
use tower_http::{compression::CompressionLayer, services::ServeDir};

use crate::app::{self, App};

mod api;
mod auth;
mod error;
mod version;

#[cfg(test)]
pub mod test;

pub fn make_router(app: App) -> Router {
	let swagger = ServeDir::new(&app.swagger_dir_path);

	let static_files = Router::new()
		.nest_service("/", ServeDir::new(&app.web_dir_path))
		.layer(CompressionLayer::new());

	Router::new()
		.nest("/api", api::router())
		.with_state(app.clone())
		.nest_service("/swagger", swagger)
		.nest("/", static_files)
}

pub async fn launch(app: App) -> Result<(), std::io::Error> {
	let port = app.port;
	let router = make_router(app);
	let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
	axum::serve(listener, router).await?;
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

impl FromRef<App> for app::lastfm::Manager {
	fn from_ref(app: &App) -> Self {
		app.lastfm_manager.clone()
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

impl FromRef<App> for app::user::Manager {
	fn from_ref(app: &App) -> Self {
		app.user_manager.clone()
	}
}

impl FromRef<App> for app::settings::Manager {
	fn from_ref(app: &App) -> Self {
		app.settings_manager.clone()
	}
}

impl FromRef<App> for app::thumbnail::Manager {
	fn from_ref(app: &App) -> Self {
		app.thumbnail_manager.clone()
	}
}

impl FromRef<App> for app::vfs::Manager {
	fn from_ref(app: &App) -> Self {
		app.vfs_manager.clone()
	}
}
