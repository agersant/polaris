use axum::{
	extract::{Path, State},
	routing::{delete, get, post, put},
	Json, Router,
};

use crate::{
	app::{config, ddns, index, settings, user, vfs, App},
	server::{dto, error::APIError},
};

use super::auth::{AdminRights, Auth};

pub fn router() -> Router<App> {
	Router::new()
		.route("/version", get(get_version))
		.route("/initial_setup", get(get_initial_setup))
		.route("/config", put(put_config))
		.route("/settings", get(get_settings))
		.route("/settings", put(put_settings))
		.route("/mount_dirs", get(get_mount_dirs))
		.route("/mount_dirs", put(put_mount_dirs))
		.route("/ddns", get(get_ddns))
		.route("/ddns", put(put_ddns))
		.route("/auth", post(post_auth))
		.route("/user", post(post_user))
		.route("/user/:name", delete(delete_user))
		.route("/user/:name", put(put_user))
		.route("/users", get(get_users))
		.route("/preferences", get(get_preferences))
		.route("/preferences", put(put_preferences))
		.route("/trigger_index", post(post_trigger_index))
}

async fn get_version() -> Json<dto::Version> {
	let current_version = dto::Version {
		major: dto::API_MAJOR_VERSION,
		minor: dto::API_MINOR_VERSION,
	};
	Json(current_version)
}

async fn get_initial_setup(
	State(user_manager): State<user::Manager>,
) -> Result<Json<dto::InitialSetup>, APIError> {
	let initial_setup = {
		let users = user_manager.list().await?;
		let has_any_admin = users.iter().any(|u| u.is_admin());
		dto::InitialSetup {
			has_any_users: has_any_admin,
		}
	};
	Ok(Json(initial_setup))
}

async fn put_config(
	_admin_rights: AdminRights,
	State(config_manager): State<config::Manager>,
	Json(config): Json<dto::Config>,
) -> Result<(), APIError> {
	config_manager.apply(&config.into()).await?;
	Ok(())
}

async fn get_settings(
	State(settings_manager): State<settings::Manager>,
	_admin_rights: AdminRights,
) -> Result<Json<dto::Settings>, APIError> {
	let settings = settings_manager.read().await?;
	Ok(Json(settings.into()))
}

async fn put_settings(
	_admin_rights: AdminRights,
	State(settings_manager): State<settings::Manager>,
	Json(new_settings): Json<dto::NewSettings>,
) -> Result<(), APIError> {
	settings_manager
		.amend(&new_settings.to_owned().into())
		.await?;
	Ok(())
}

async fn get_mount_dirs(
	_admin_rights: AdminRights,
	State(vfs_manager): State<vfs::Manager>,
) -> Result<Json<Vec<dto::MountDir>>, APIError> {
	let mount_dirs = vfs_manager.mount_dirs().await?;
	let mount_dirs = mount_dirs.into_iter().map(|m| m.into()).collect();
	Ok(Json(mount_dirs))
}

async fn put_mount_dirs(
	_admin_rights: AdminRights,
	State(vfs_manager): State<vfs::Manager>,
	new_mount_dirs: Json<Vec<dto::MountDir>>,
) -> Result<(), APIError> {
	let new_mount_dirs: Vec<vfs::MountDir> =
		new_mount_dirs.iter().cloned().map(|m| m.into()).collect();
	vfs_manager.set_mount_dirs(&new_mount_dirs).await?;
	Ok(())
}

async fn get_ddns(
	_admin_rights: AdminRights,
	State(ddns_manager): State<ddns::Manager>,
) -> Result<Json<dto::DDNSConfig>, APIError> {
	let ddns_config = ddns_manager.config().await?;
	Ok(Json(ddns_config.into()))
}

async fn put_ddns(
	_admin_rights: AdminRights,
	State(ddns_manager): State<ddns::Manager>,
	Json(new_ddns_config): Json<dto::DDNSConfig>,
) -> Result<(), APIError> {
	ddns_manager.set_config(&new_ddns_config.into()).await?;
	Ok(())
}

async fn post_auth(
	State(user_manager): State<user::Manager>,
	credentials: Json<dto::Credentials>,
) -> Result<Json<dto::Authorization>, APIError> {
	let username = credentials.username.clone();

	let user::AuthToken(token) = user_manager
		.login(&credentials.username, &credentials.password)
		.await?;
	let is_admin = user_manager.is_admin(&credentials.username).await?;

	let authorization = dto::Authorization {
		username: username.clone(),
		token,
		is_admin,
	};

	Ok(Json(authorization))
}

async fn get_users(
	_admin_rights: AdminRights,
	State(user_manager): State<user::Manager>,
) -> Result<Json<Vec<dto::User>>, APIError> {
	let users = user_manager.list().await?;
	let users = users.into_iter().map(|u| u.into()).collect();
	Ok(Json(users))
}

async fn post_user(
	_admin_rights: AdminRights,
	State(user_manager): State<user::Manager>,
	Json(new_user): Json<dto::NewUser>,
) -> Result<(), APIError> {
	user_manager.create(&new_user.into()).await?;
	Ok(())
}

async fn put_user(
	admin_rights: AdminRights,
	State(user_manager): State<user::Manager>,
	Path(name): Path<String>,
	user_update: Json<dto::UserUpdate>,
) -> Result<(), APIError> {
	if let Some(auth) = &admin_rights.get_auth() {
		if auth.get_username() == name.as_str() && user_update.new_is_admin == Some(false) {
			return Err(APIError::OwnAdminPrivilegeRemoval);
		}
	}

	if let Some(password) = &user_update.new_password {
		user_manager.set_password(&name, password).await?;
	}

	if let Some(is_admin) = &user_update.new_is_admin {
		user_manager.set_is_admin(&name, *is_admin).await?;
	}

	Ok(())
}

async fn delete_user(
	admin_rights: AdminRights,
	State(user_manager): State<user::Manager>,
	Path(name): Path<String>,
) -> Result<(), APIError> {
	if let Some(auth) = &admin_rights.get_auth() {
		if auth.get_username() == name.as_str() {
			return Err(APIError::DeletingOwnAccount);
		}
	}
	user_manager.delete(&name).await?;
	Ok(())
}

async fn get_preferences(
	auth: Auth,
	State(user_manager): State<user::Manager>,
) -> Result<Json<user::Preferences>, APIError> {
	let preferences = user_manager.read_preferences(auth.get_username()).await?;
	Ok(Json(preferences))
}

async fn put_preferences(
	auth: Auth,
	State(user_manager): State<user::Manager>,
	Json(preferences): Json<user::Preferences>,
) -> Result<(), APIError> {
	user_manager
		.write_preferences(auth.get_username(), &preferences)
		.await?;
	Ok(())
}

async fn post_trigger_index(
	_admin_rights: AdminRights,
	State(index): State<index::Index>,
) -> Result<(), APIError> {
	index.trigger_reindex();
	Ok(())
}
