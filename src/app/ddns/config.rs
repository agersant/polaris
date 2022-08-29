use serde::{Deserialize, Serialize};

use crate::db::ddns_config;

#[derive(Clone, Debug, Deserialize, Insertable, PartialEq, Eq, Queryable, Serialize)]
#[diesel(table_name = ddns_config)]
pub struct Config {
	pub host: String,
	pub username: String,
	pub password: String,
}
