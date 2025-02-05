use log::{debug, error};
use std::time::Duration;

use crate::app::{config, Error};

#[derive(Clone)]
pub struct Manager {
	config_manager: config::Manager,
}

impl Manager {
	pub fn new(config_manager: config::Manager) -> Self {
		Self { config_manager }
	}

	pub async fn update_ddns(&self) -> Result<(), Error> {
		let url = self.config_manager.get_ddns_update_url().await;
		let Some(url) = url else {
			debug!("Skipping DDNS update because credentials are missing");
			return Ok(());
		};

		let response = ureq::get(&url.to_string()).call();

		match response {
			Ok(_) => Ok(()),
			Err(ureq::Error::Status(code, _)) => Err(Error::UpdateQueryFailed(code)),
			Err(ureq::Error::Transport(_)) => Err(Error::UpdateQueryTransport),
		}
	}

	pub fn begin_periodic_updates(&self) {
		tokio::spawn({
			let ddns = self.clone();
			async move {
				loop {
					if let Err(e) = ddns.update_ddns().await {
						error!("Dynamic DNS update error: {:?}", e);
					}
					tokio::time::sleep(Duration::from_secs(60 * 30)).await;
				}
			}
		});
	}
}
