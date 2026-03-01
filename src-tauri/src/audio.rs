use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rubato::{FftFixedIn, Resampler};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

/// Staging buffer + resampler that accumulates raw mono samples from the callback
/// and drains fixed-size chunks into the rubato FftFixedIn resampler.
///
/// rubato FftFixedIn requires a fixed input chunk size. cpal callbacks deliver
/// variable-length buffers, so this struct bridges them.
struct ResamplingState {
    resampler: FftFixedIn<f32>,
    staging: Vec<f32>,
    chunk_size: usize,
}

impl ResamplingState {
    fn new(in_rate: usize) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let chunk_size = 1024; // ~64ms at 16kHz
        let resampler = FftFixedIn::<f32>::new(
            in_rate,
            16000,
            chunk_size,
            2, // sub_chunks
            1, // output channels (mono)
        )?;
        Ok(Self {
            resampler,
            staging: Vec::new(),
            chunk_size,
        })
    }

    /// Push mono samples and drain complete chunks through the resampler.
    /// Returns resampled 16kHz output samples.
    fn push(&mut self, samples: &[f32]) -> Vec<f32> {
        self.staging.extend_from_slice(samples);
        let mut output = Vec::new();

        while self.staging.len() >= self.chunk_size {
            let chunk: Vec<Vec<f32>> = vec![self.staging.drain(..self.chunk_size).collect()];
            if let Ok(out) = self.resampler.process(&chunk, None) {
                output.extend_from_slice(&out[0]);
            }
        }

        output
    }

    /// Flush remaining staging samples by zero-padding to chunk_size, then process.
    /// Returns the final resampled output. Resets internal state.
    fn flush(&mut self) -> Vec<f32> {
        if self.staging.is_empty() {
            return Vec::new();
        }

        let remaining = self.staging.len();
        let mut padded = self.staging.clone();
        padded.resize(self.chunk_size, 0.0);
        self.staging.clear();

        let chunk = vec![padded];
        let mut output = Vec::new();
        if let Ok(out) = self.resampler.process(&chunk, None) {
            // Only take samples corresponding to actual (non-padded) input
            let out_samples = (remaining * 16000 + self.chunk_size - 1) / self.chunk_size;
            let take = out_samples.min(out[0].len());
            output.extend_from_slice(&out[0][..take]);
        }

        output
    }
}

/// Persistent microphone capture state.
///
/// `_stream` must stay alive for the duration of capture — dropping it stops the stream.
/// `recording` is an atomic flag toggled by Tauri commands.
/// `buffer` accumulates 16kHz mono samples while recording is active.
/// `resampling` is the staging + resampler state, locked separately from `buffer`
/// so the callback can push samples without contending on the main buffer.
pub struct AudioCapture {
    _stream: cpal::Stream,
    pub recording: Arc<AtomicBool>,
    pub buffer: Arc<Mutex<Vec<f32>>>,
    /// Resampling state for flush-on-stop. Guarded by Mutex because the audio
    /// callback (background thread) writes to it, and Tauri commands read it.
    resampling: Arc<Mutex<ResamplingState>>,
}

// SAFETY: cpal::Stream is Send but not Sync.
// AudioCapture is stored as Tauri managed state which requires Send + Sync.
// We never share a &cpal::Stream reference across threads — only Arc clones of
// the AtomicBool and Mutex-guarded fields are accessed from other threads.
unsafe impl Sync for AudioCapture {}

/// Mutex wrapper around AudioCapture for runtime device switching.
///
/// The outer Mutex guards the entire AudioCapture for replacement (mic switch).
/// The inner Mutex inside AudioCapture guards the audio buffer for the callback.
/// These two locks serve different purposes and do not nest.
pub struct AudioCaptureMutex(pub std::sync::Mutex<AudioCapture>);

// SAFETY: AudioCaptureMutex wraps AudioCapture (which is already Sync via unsafe impl).
// The Mutex ensures exclusive access for replacement operations.
unsafe impl Sync for AudioCaptureMutex {}

/// Internal: build an input stream from a specific device.
///
/// Extracted to share logic between `start_persistent_stream` and
/// `start_persistent_stream_with_device`. Never call with a blocking lock held.
fn build_stream_from_device(
    device: cpal::Device,
) -> Result<AudioCapture, Box<dyn std::error::Error + Send + Sync>> {
    let device_name = device
        .description()
        .map(|d| d.name().to_string())
        .unwrap_or_else(|_| "<unknown>".to_string());

    let config = device.default_input_config()?;
    let native_rate = config.sample_rate() as usize;
    let channels = config.channels() as usize;

    log::info!(
        "Audio device: '{}', native rate: {} Hz, channels: {}",
        device_name,
        native_rate,
        channels
    );

    let recording = Arc::new(AtomicBool::new(false));
    let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let resampling = Arc::new(Mutex::new(ResamplingState::new(native_rate)?));

    let recording_cb = recording.clone();
    let buffer_cb = buffer.clone();
    let resampling_cb = resampling.clone();

    let stream = device.build_input_stream(
        &config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            // Discard samples when not recording
            if !recording_cb.load(Ordering::Relaxed) {
                return;
            }

            // Downmix all channels to mono
            let mono: Vec<f32> = data
                .chunks(channels)
                .map(|frame| frame.iter().sum::<f32>() / channels as f32)
                .collect();

            // Resample through staging buffer. Use try_lock to never block the
            // audio callback thread (Pitfall 2 from RESEARCH.md).
            if let Ok(mut rs) = resampling_cb.try_lock() {
                let resampled = rs.push(&mono);
                if !resampled.is_empty() {
                    if let Ok(mut buf) = buffer_cb.try_lock() {
                        buf.extend_from_slice(&resampled);
                    }
                }
            }
        },
        |err| log::error!("Audio stream error: {}", err),
        None,
    )?;

    stream.play()?;

    log::info!(
        "Audio stream started: {} Hz, {} ch -> 16kHz mono (persistent)",
        native_rate,
        channels
    );

    Ok(AudioCapture {
        _stream: stream,
        recording,
        buffer,
        resampling,
    })
}

/// Start a persistent microphone capture stream using the system default device.
///
/// The stream runs continuously from app startup. Audio is discarded unless the
/// `recording` flag is set to `true`. When recording, samples are downmixed to
/// mono, resampled to 16kHz, and appended to `buffer`.
pub fn start_persistent_stream(
) -> Result<AudioCapture, Box<dyn std::error::Error + Send + Sync>> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("No default input device found")?;
    build_stream_from_device(device)
}

/// Start a persistent microphone capture stream using a specific device.
///
/// Same as `start_persistent_stream()` but accepts a `cpal::Device` directly.
/// Used by `set_microphone` to restart the stream with a user-selected device.
pub fn start_persistent_stream_with_device(
    device: cpal::Device,
) -> Result<AudioCapture, Box<dyn std::error::Error + Send + Sync>> {
    build_stream_from_device(device)
}

/// Write a slice of 32-bit float samples to a WAV file at 16kHz mono.
pub fn write_wav(path: &str, samples: &[f32]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    for &sample in samples {
        writer.write_sample(sample)?;
    }
    writer.finalize()?;
    Ok(())
}

impl AudioCapture {
    /// Clear the accumulated audio buffer and reset resampler staging state.
    /// Call before starting a new recording to avoid previous audio bleeding in.
    pub fn clear_buffer(&self) {
        if let Ok(mut buf) = self.buffer.lock() {
            buf.clear();
        }
        // Reset resampler staging by replacing with a fresh instance using the
        // same native rate as the original. Since we cannot easily retrieve the
        // original native_rate after construction, we rely on the caller to
        // re-initialize if needed. The staging Vec clear is the critical part.
        if let Ok(mut rs) = self.resampling.lock() {
            rs.staging.clear();
        }
    }

    /// Flush the resampler staging buffer and append any remaining samples to
    /// the main buffer. Returns the total number of 16kHz samples in the buffer.
    pub fn flush_and_stop(&self) -> usize {
        self.recording.store(false, Ordering::Relaxed);

        // Flush remaining samples from the staging buffer
        if let Ok(mut rs) = self.resampling.lock() {
            let tail = rs.flush();
            if !tail.is_empty() {
                if let Ok(mut buf) = self.buffer.lock() {
                    buf.extend_from_slice(&tail);
                }
            }
        }

        self.buffer
            .lock()
            .map(|b| b.len())
            .unwrap_or(0)
    }

    /// Get a copy of all buffered 16kHz mono samples.
    pub fn get_buffer(&self) -> Vec<f32> {
        self.buffer
            .lock()
            .map(|b| b.clone())
            .unwrap_or_default()
    }
}
