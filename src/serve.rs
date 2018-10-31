use iron::headers::{
	AcceptRanges, ByteRangeSpec, ContentLength, ContentRange, ContentRangeSpec, Range, RangeUnit,
};
use iron::modifier::Modifier;
use iron::modifiers::Header;
use iron::prelude::*;
use iron::response::WriteBody;
use iron::status::{self, Status};
use std::cmp;
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

use crate::errors::{Error, ErrorKind};

pub fn deliver(path: &Path, range_header: Option<&Range>) -> IronResult<Response> {
	match fs::metadata(path) {
		Ok(meta) => meta,
		Err(e) => {
			let status = match e.kind() {
				io::ErrorKind::NotFound => status::NotFound,
				io::ErrorKind::PermissionDenied => status::Forbidden,
				_ => status::InternalServerError,
			};
			return Err(IronError::new(e, status));
		}
	};

	let accept_range_header = Header(AcceptRanges(vec![RangeUnit::Bytes]));
	let range_header = range_header.cloned();

	match range_header {
		None => Ok(Response::with((status::Ok, path, accept_range_header))),
		Some(range) => match range {
			Range::Bytes(vec_range) => {
				if let Ok(partial_file) = PartialFile::from_path(path, vec_range) {
					Ok(Response::with((
						status::Ok,
						partial_file,
						accept_range_header,
					)))
				} else {
					Err(Error::from(ErrorKind::FileNotFound).into())
				}
			}
			_ => Ok(Response::with(status::RangeNotSatisfiable)),
		},
	}
}

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

pub struct PartialFile {
	file: File,
	range: PartialFileRange,
}

impl From<Vec<ByteRangeSpec>> for PartialFileRange {
	fn from(v: Vec<ByteRangeSpec>) -> PartialFileRange {
		match v.into_iter().next() {
			None => PartialFileRange::AllFrom(0),
			Some(byte_range) => PartialFileRange::from(byte_range),
		}
	}
}

impl PartialFile {
	pub fn new<Range>(file: File, range: Range) -> PartialFile
	where
		Range: Into<PartialFileRange>,
	{
		let range = range.into();
		PartialFile { file, range }
	}

	pub fn from_path<P: AsRef<Path>, Range>(path: P, range: Range) -> Result<PartialFile, io::Error>
	where
		Range: Into<PartialFileRange>,
	{
		let file = File::open(path.as_ref())?;
		Ok(Self::new(file, range))
	}
}

impl Modifier<Response> for PartialFile {
	fn modify(self, res: &mut Response) {
		use self::PartialFileRange::*;
		let metadata: Option<_> = self.file.metadata().ok();
		let file_length: Option<u64> = metadata.map(|m| m.len());
		let range: Option<(u64, u64)> = match (self.range, file_length) {
			(FromTo(from, to), Some(file_length)) => {
				if from <= to && from < file_length {
					Some((from, cmp::min(to, file_length - 1)))
				} else {
					None
				}
			}
			(AllFrom(from), Some(file_length)) => {
				if from < file_length {
					Some((from, file_length - 1))
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
		};
		if let Some(range) = range {
			let content_range = ContentRange(ContentRangeSpec::Bytes {
				range: Some(range),
				instance_length: file_length,
			});
			let content_len = range.1 - range.0 + 1;
			res.headers.set(ContentLength(content_len));
			res.headers.set(content_range);
			let partial_content = PartialContentBody {
				file: self.file,
				offset: range.0,
				len: content_len,
			};
			res.status = Some(Status::PartialContent);
			res.body = Some(Box::new(partial_content));
		} else {
			if let Some(file_length) = file_length {
				res.headers.set(ContentRange(ContentRangeSpec::Bytes {
					range: None,
					instance_length: Some(file_length),
				}));
			};
			res.status = Some(Status::RangeNotSatisfiable);
		}
	}
}

struct PartialContentBody {
	pub file: File,
	pub offset: u64,
	pub len: u64,
}

impl WriteBody for PartialContentBody {
	fn write_body(&mut self, res: &mut Write) -> io::Result<()> {
		self.file.seek(SeekFrom::Start(self.offset))?;
		let mut limiter = <File as Read>::by_ref(&mut self.file).take(self.len);
		io::copy(&mut limiter, res).map(|_| ())
	}
}
