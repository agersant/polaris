use axum::{extract::FromRef, Router};
use tower_http::services::ServeDir;

use crate::app::{self, App};

mod api;
mod auth;
mod error;

#[cfg(test)]
pub mod test;

pub fn make_router(app: App) -> Router {
	Router::new()
		.nest("/api", api::router())
		.with_state(app.clone())
		.nest_service("/swagger", ServeDir::new(app.swagger_dir_path))
		.nest_service("/", ServeDir::new(app.web_dir_path))
}

pub async fn launch(app: App) -> Result<(), std::io::Error> {
	let port = app.port;
	let router = make_router(app);
	let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
	axum::serve(listener, router).await?;
	Ok(())
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

impl FromRef<App> for app::index::Index {
	fn from_ref(app: &App) -> Self {
		app.index.clone()
	}
}

impl FromRef<App> for app::lastfm::Manager {
	fn from_ref(app: &App) -> Self {
		app.lastfm_manager.clone()
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

impl FromRef<App> for app::scanner::Scanner {
	fn from_ref(app: &App) -> Self {
		app.scanner.clone()
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
