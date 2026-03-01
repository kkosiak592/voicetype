use std::f32::consts::PI;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use rustfft::{FftPlanner, num_complex::Complex};
use tauri::Emitter;

/// Payload emitted on every tick with both RMS level and 16-bin FFT spectrum.
#[derive(Clone, serde::Serialize)]
struct PillLevelPayload {
    rms: f32,
    bins: Vec<f32>,
}

/// Start a ~30fps loop that reads the audio buffer, computes RMS + FFT bins,
/// and emits a PillLevelPayload to the pill window.
///
/// The loop runs until `active` is set to false. Call this when entering
/// RECORDING state and stop it when leaving RECORDING state.
pub fn start_level_stream(
    app: tauri::AppHandle,
    buffer: Arc<std::sync::Mutex<Vec<f32>>>,
    active: Arc<AtomicBool>,
) {
    // Create FFT planner and plan once — reused across all ticks (Pitfall 1 avoidance)
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(256);

    tauri::async_runtime::spawn(async move {
        while active.load(Ordering::Relaxed) {
            let (level, bins) = if let Ok(buf) = buffer.try_lock() {
                let rms = compute_rms(&buf, 512);
                let bins = compute_fft_bins(&buf, 16, rms, &fft);
                (rms, bins)
            } else {
                (0.0, vec![0.0; 16])
            };
            let _ = app.emit_to("pill", "pill-level", PillLevelPayload { rms: level, bins });
            tokio::time::sleep(std::time::Duration::from_millis(33)).await;
        }
        // Send a final zero level when stopping
        let _ = app.emit_to("pill", "pill-level", PillLevelPayload {
            rms: 0.0,
            bins: vec![0.0; 16],
        });
    });
}

/// Compute normalized RMS from the last `window` samples of the buffer.
///
/// Returns 0.0-1.0 where typical speech is 0.45-1.0.
/// Uses a 15x multiplier: speech RMS is usually 0.01-0.1, so * 15 normalizes
/// to a higher range (0.15-1.0) for more prominent bar reactivity.
fn compute_rms(buf: &[f32], window: usize) -> f32 {
    if buf.is_empty() {
        return 0.0;
    }
    let n = buf.len().min(window);
    let tail = &buf[buf.len() - n..];
    let mean_sq: f32 = tail.iter().map(|&s| s * s).sum::<f32>() / n as f32;
    (mean_sq.sqrt() * 15.0).min(1.0)
}

/// Compute `n_bins` normalized FFT magnitude bins from the last 256 samples.
///
/// - Applies a Hann window to reduce spectral leakage
/// - Runs a 256-point forward FFT
/// - Takes first 128 magnitudes (meaningful spectrum — second half is mirror)
/// - Bins into `n_bins` groups (128 / n_bins samples per bin), taking max per group
/// - Normalizes all bins to 0-1 range by peak magnitude
/// - Gates on RMS: returns all-zero bins if rms < 0.02 (avoids ambient noise visual)
fn compute_fft_bins(
    buf: &[f32],
    n_bins: usize,
    rms: f32,
    fft: &Arc<dyn rustfft::Fft<f32>>,
) -> Vec<f32> {
    const FFT_SIZE: usize = 256;

    // Gate on RMS — suppress visual noise from ambient mic noise (Pitfall 6)
    if rms < 0.02 || buf.len() < FFT_SIZE {
        return vec![0.0; n_bins];
    }

    // Take last FFT_SIZE samples and apply Hann window
    let start = buf.len() - FFT_SIZE;
    let mut input: Vec<Complex<f32>> = buf[start..start + FFT_SIZE]
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            let window = 0.5 - 0.5 * (2.0 * PI * i as f32 / (FFT_SIZE - 1) as f32).cos();
            Complex { re: s * window, im: 0.0 }
        })
        .collect();

    // Run FFT in-place
    fft.process(&mut input);

    // Take first 128 magnitudes (meaningful half of spectrum)
    let half = FFT_SIZE / 2;
    let magnitudes: Vec<f32> = input[..half].iter().map(|c| c.norm()).collect();

    // Bin into n_bins groups — take max magnitude per group
    let samples_per_bin = half / n_bins;
    let mut bins: Vec<f32> = (0..n_bins)
        .map(|b| {
            let start = b * samples_per_bin;
            let end = (start + samples_per_bin).min(half);
            magnitudes[start..end]
                .iter()
                .cloned()
                .fold(0.0_f32, f32::max)
        })
        .collect();

    // Normalize bins to 0-1 range by peak
    let peak = bins.iter().cloned().fold(0.0_f32, f32::max);
    if peak > 0.0 {
        bins.iter_mut().for_each(|b| *b /= peak);
    }

    bins
}
