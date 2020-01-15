use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct Version {
	pub major: i32,
	pub minor: i32,
}
