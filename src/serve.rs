use log::warn;
use rocket;
use rocket::http::hyper::header::*;
use rocket::http::Status;
use rocket::response::{self, Responder};
use rocket::Response;
use std::cmp;
use std::convert::From;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::str::FromStr;

#[derive(Debug)]
pub enum PartialFileRange {
	AllFrom(u64),
	FromTo(u64, u64),
	Last(u64),
}

impl From<ByteRangeSpec> for PartialFileRange {
	fn from(b: ByteRangeSpec) -> PartialFileRange {
		match b {
			ByteRangeSpec::AllFrom(from) => PartialFileRange::AllFrom(from),
			ByteRangeSpec::FromTo(from, to) => PartialFileRange::FromTo(from, to),
			ByteRangeSpec::Last(last) => PartialFileRange::Last(last),
		}
	}
}

impl From<Vec<ByteRangeSpec>> for PartialFileRange {
	fn from(v: Vec<ByteRangeSpec>) -> PartialFileRange {
		match v.into_iter().next() {
			None => PartialFileRange::AllFrom(0),
			Some(byte_range) => PartialFileRange::from(byte_range),
		}
	}
}

pub struct RangeResponder<R> {
	original: R,
}

impl<'r, R: Responder<'r>> RangeResponder<R> {
	pub fn new(original: R) -> RangeResponder<R> {
		RangeResponder { original }
	}

	fn ignore_range(
		self,
		request: &rocket::request::Request<'_>,
		file_length: Option<u64>,
	) -> response::Result<'r> {
		let mut response = self.original.respond_to(request)?;
		if let Some(content_length) = file_length {
			response.set_header(ContentLength(content_length));
		}
		response.set_status(Status::Ok);
		Ok(response)
	}

	fn reject_range(self, file_length: Option<u64>) -> response::Result<'r> {
		let mut response = Response::build()
			.status(Status::RangeNotSatisfiable)
			.finalize();
		if file_length.is_some() {
			let content_range = ContentRange(ContentRangeSpec::Bytes {
				range: None,
				instance_length: file_length,
			});
			response.set_header(content_range);
		}
		response.set_status(Status::RangeNotSatisfiable);
		Ok(response)
	}
}

fn truncate_range(range: &PartialFileRange, file_length: &Option<u64>) -> Option<(u64, u64)> {
	use self::PartialFileRange::*;

	match (range, file_length) {
		(FromTo(from, to), Some(file_length)) => {
			if from <= to && from < file_length {
				Some((*from, cmp::min(*to, file_length - 1)))
			} else {
				None
			}
		}
		(AllFrom(from), Some(file_length)) => {
			if from < file_length {
				Some((*from, file_length - 1))
			} else {
				None
			}
		}
		(Last(last), Some(file_length)) => {
			if last < file_length {
				Some((file_length - last, file_length - 1))
			} else {
				Some((0, file_length - 1))
			}
		}
		(_, None) => None,
	}
}

impl<'r> Responder<'r> for RangeResponder<File> {
	fn respond_to(mut self, request: &rocket::request::Request<'_>) -> response::Result<'r> {
		let metadata: Option<_> = self.original.metadata().ok();
		let file_length: Option<u64> = metadata.map(|m| m.len());

		let range_header = request.headers().get_one("Range");
		let range_header = match range_header {
			None => return self.ignore_range(request, file_length),
			Some(h) => h,
		};

		let vec_range = match Range::from_str(range_header) {
			Ok(Range::Bytes(v)) => v,
			_ => {
				warn!(
					"Ignoring range header that could not be parse {:?}, file length is {:?}",
					range_header, file_length
				);
				return self.ignore_range(request, file_length);
			}
		};

		let partial_file_range = match vec_range.into_iter().next() {
			None => PartialFileRange::AllFrom(0),
			Some(byte_range) => PartialFileRange::from(byte_range),
		};

		let range: Option<(u64, u64)> = truncate_range(&partial_file_range, &file_length);

		if let Some((from, to)) = range {
			let content_range = ContentRange(ContentRangeSpec::Bytes {
				range: range,
				instance_length: file_length,
			});
			let content_len = to - from + 1;

			match self.original.seek(SeekFrom::Start(from)) {
				Ok(_) => (),
				Err(_) => return Err(rocket::http::Status::InternalServerError),
			}
			let partial_original = self.original.take(content_len);
			let response = Response::build()
				.status(Status::PartialContent)
				.header(ContentLength(content_len))
				.header(content_range)
				.streamed_body(partial_original)
				.finalize();

			Ok(response)
		} else {
			warn!(
				"Rejecting unsatisfiable range header {:?}, file length is {:?}",
				&partial_file_range, &file_length
			);
			self.reject_range(file_length)
		}
	}
}
