//! Audio file decoding: WAV / MP3 / FLAC → mono f32 @ 22050 Hz.

use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use super::{EarError, SAMPLE_RATE};

/// Decode an audio file to mono f32 samples at [`SAMPLE_RATE`] Hz.
pub fn decode_audio(path: &Path) -> Result<Vec<f32>, EarError> {
    let file = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| EarError::Decode(e.to_string()))?;

    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| EarError::Decode("no audio track found".into()))?;

    let track_id = track.id;
    let codec_params = track.codec_params.clone();
    let source_rate = codec_params.sample_rate.unwrap_or(44100);
    let channels = codec_params.channels.map(|c| c.count()).unwrap_or(1);

    let mut decoder = symphonia::default::get_codecs()
        .make(&codec_params, &DecoderOptions::default())
        .map_err(|e| EarError::Decode(e.to_string()))?;

    let mut all_samples: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(_) => break,
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let spec = *decoded.spec();
        let n_frames = decoded.capacity();
        let mut sample_buf = SampleBuffer::<f32>::new(n_frames as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);

        let interleaved = sample_buf.samples();
        // Mix to mono
        for frame in interleaved.chunks(channels) {
            let mono: f32 = frame.iter().sum::<f32>() / channels as f32;
            all_samples.push(mono);
        }
    }

    if all_samples.is_empty() {
        return Err(EarError::EmptyAudio);
    }

    // Resample to SAMPLE_RATE if needed
    if source_rate != SAMPLE_RATE {
        all_samples = resample(&all_samples, source_rate, SAMPLE_RATE);
    }

    Ok(all_samples)
}

/// High-quality resampling using the rubato crate (sinc interpolation).
fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    use rubato::{FftFixedIn, Resampler};

    let ratio = to_rate as f64 / from_rate as f64;
    let chunk_size = 1024;

    let mut resampler =
        FftFixedIn::<f32>::new(from_rate as usize, to_rate as usize, chunk_size, 2, 1)
            .expect("failed to create resampler");

    let mut output = Vec::with_capacity((samples.len() as f64 * ratio) as usize + chunk_size);

    let mut pos = 0;
    while pos + chunk_size <= samples.len() {
        let chunk = vec![samples[pos..pos + chunk_size].to_vec()];
        if let Ok(out) = resampler.process(&chunk, None) {
            if let Some(ch) = out.first() {
                output.extend_from_slice(ch);
            }
        }
        pos += chunk_size;
    }

    // Handle remaining samples by padding with zeros
    if pos < samples.len() {
        let remaining = samples.len() - pos;
        let mut padded = samples[pos..].to_vec();
        padded.resize(chunk_size, 0.0);
        let chunk = vec![padded];
        if let Ok(out) = resampler.process(&chunk, None) {
            if let Some(ch) = out.first() {
                // Only take the proportional amount
                let take = (remaining as f64 * ratio) as usize;
                let take = take.min(ch.len());
                output.extend_from_slice(&ch[..take]);
            }
        }
    }

    output
}
