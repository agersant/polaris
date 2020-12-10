#[derive(Debug, Hash)]
pub struct Options {
	pub max_dimension: u32,
	pub resize_if_almost_square: bool,
	pub pad_to_square: bool,
}

impl Default for Options {
	fn default() -> Self {
		Self {
			max_dimension: 400,
			resize_if_almost_square: true,
			pad_to_square: true,
		}
	}
}
