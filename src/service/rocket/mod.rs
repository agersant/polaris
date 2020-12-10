use anyhow::*;
use rocket;
use rocket::config::{Environment, LoggingLevel};
use rocket_contrib::serve::{Options, StaticFiles};

use crate::service;

mod api;
mod serve;

#[cfg(test)]
pub mod test;

pub fn get_server(context: service::Context) -> Result<rocket::Rocket> {
	let mut config = rocket::Config::build(Environment::Production)
		.log_level(LoggingLevel::Normal)
		.port(context.port)
		.keep_alive(0)
		.finalize()?;

	let encoded = base64::encode(&context.auth_secret);
	config.set_secret_key(encoded)?;

	let swagger_routes_rank = 0;
	let web_routes_rank = swagger_routes_rank + 1;
	let static_file_options = Options::Index | Options::NormalizeDirs;

	Ok(rocket::custom(config)
		.manage(context.db)
		.manage(context.index)
		.manage(context.playlists_manager)
		.manage(context.thumbnails_manager)
		.mount(&context.api_url, api::get_routes())
		.mount(
			&context.swagger_url,
			StaticFiles::new(context.swagger_dir_path, static_file_options)
				.rank(swagger_routes_rank),
		)
		.mount(
			&context.web_url,
			StaticFiles::new(context.web_dir_path, static_file_options).rank(web_routes_rank),
		))
}

pub fn run(context: service::Context) -> Result<()> {
	let server = get_server(context)?;
	server.launch();
	Ok(())
}
