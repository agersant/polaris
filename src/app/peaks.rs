use std::{
	hash::{DefaultHasher, Hash, Hasher},
	path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use symphonia::core::{
	audio::SampleBuffer,
	codecs::{DecoderOptions, CODEC_TYPE_NULL},
	formats::FormatOptions,
	io::{MediaSourceStream, MediaSourceStreamOptions},
	meta::MetadataOptions,
	probe::Hint,
};
use tokio::{io::AsyncWriteExt, task::spawn_blocking};

use crate::app::Error;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Peaks {
	pub interleaved: Vec<u8>,
}

#[derive(Clone)]
pub struct Manager {
	peaks_dir_path: PathBuf,
}

impl Manager {
	pub fn new(peaks_dir_path: PathBuf) -> Self {
		Self { peaks_dir_path }
	}

	pub async fn get_peaks(&self, audio_path: &Path) -> Result<Peaks, Error> {
		match self.read_from_cache(audio_path).await {
			Ok(Some(peaks)) => Ok(peaks),
			_ => self.read_from_source(audio_path).await,
		}
	}

	fn get_peaks_path(&self, audio_path: &Path) -> PathBuf {
		let hash = Manager::hash(audio_path);
		let mut peaks_path = self.peaks_dir_path.clone();
		peaks_path.push(format!("{}.peaks", hash));
		peaks_path
	}

	async fn read_from_cache(&self, audio_path: &Path) -> Result<Option<Peaks>, Error> {
		let peaks_path = self.get_peaks_path(audio_path);
		if peaks_path.exists() {
			let serialized = tokio::fs::read(&peaks_path)
				.await
				.map_err(|e| Error::Io(peaks_path.clone(), e))?;
			let peaks =
				bitcode::deserialize::<Peaks>(&serialized).map_err(Error::PeaksDeserialization)?;
			Ok(Some(peaks))
		} else {
			Ok(None)
		}
	}

	async fn read_from_source(&self, audio_path: &Path) -> Result<Peaks, Error> {
		let peaks = spawn_blocking({
			let audio_path = audio_path.to_owned();
			move || compute_peaks(&audio_path)
		})
		.await??;

		let serialized = bitcode::serialize(&peaks).map_err(Error::PeaksSerialization)?;

		tokio::fs::create_dir_all(&self.peaks_dir_path)
			.await
			.map_err(|e| Error::Io(self.peaks_dir_path.clone(), e))?;

		let path = self.get_peaks_path(audio_path);
		let mut out_file = tokio::fs::File::create(&path)
			.await
			.map_err(|e| Error::Io(path.clone(), e))?;

		out_file
			.write_all(&serialized)
			.await
			.map_err(|e| Error::Io(path.clone(), e))?;

		Ok(peaks)
	}

	fn hash(path: &Path) -> u64 {
		let mut hasher = DefaultHasher::new();
		path.hash(&mut hasher);
		hasher.finish()
	}
}

fn compute_peaks(audio_path: &Path) -> Result<Peaks, Error> {
	let peaks_per_minute = 4000;

	let file =
		std::fs::File::open(audio_path).map_err(|e| Error::Io(audio_path.to_owned(), e))?;
	let media_source = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());

	let mut peaks = Peaks::default();
	peaks.interleaved.reserve(5 * peaks_per_minute);

	let mut format = symphonia::default::get_probe()
		.format(
			&Hint::new(),
			media_source,
			&FormatOptions::default(),
			&MetadataOptions::default(),
		)
		.map_err(Error::MediaProbeError)?
		.format;

	let track = format
		.tracks()
		.iter()
		.find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
		.ok_or_else(|| Error::MediaEmpty(audio_path.to_owned()))?;

	let track_id = track.id;

	let mut decoder = symphonia::default::get_codecs()
		.make(&track.codec_params, &DecoderOptions::default())
		.map_err(Error::MediaDecoderError)?;

	let (mut min, mut max) = (u8::MAX, u8::MIN);
	let mut num_ingested = 0;

	loop {
		let packet = match format.next_packet() {
			Ok(packet) => packet,
			Err(symphonia::core::errors::Error::IoError(e))
				if e.kind() == std::io::ErrorKind::UnexpectedEof =>
			{
				break;
			}
			Err(e) => return Err(Error::MediaPacketError(e)),
		};

		if packet.track_id() != track_id {
			continue;
		}

		let decoded = match decoder.decode(&packet) {
			Ok(d) => d,
			Err(_) => continue,
		};

		let num_channels = decoded.spec().channels.count();
		let sample_rate = decoded.spec().rate;
		let num_samples_per_peak =
			((sample_rate as f32) * 60.0 / (peaks_per_minute as f32)).round() as usize;

		let mut buffer = SampleBuffer::<u8>::new(decoded.capacity() as u64, *decoded.spec());
		buffer.copy_interleaved_ref(decoded);
		for samples in buffer.samples().chunks_exact(num_channels) {
			// Merge channels into mono signal
			let mut mono: u32 = 0;
			for sample in samples {
				mono += *sample as u32;
			}
			mono /= samples.len() as u32;

			min = u8::min(min, mono as u8);
			max = u8::max(max, mono as u8);
			num_ingested += 1;

			if num_ingested >= num_samples_per_peak {
				peaks.interleaved.push(min);
				peaks.interleaved.push(max);
				(min, max) = (u8::MAX, u8::MIN);
				num_ingested = 0;
			}
		}
	}

	Ok(peaks)
}
