use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Emitter;

/// Start a ~30fps loop that reads the audio buffer, computes RMS, and emits
/// normalized level (0.0-1.0) to the pill window.
///
/// The loop runs until `active` is set to false. Call this when entering
/// RECORDING state and stop it when leaving RECORDING state.
pub fn start_level_stream(
    app: tauri::AppHandle,
    buffer: Arc<std::sync::Mutex<Vec<f32>>>,
    active: Arc<AtomicBool>,
) {
    tauri::async_runtime::spawn(async move {
        while active.load(Ordering::Relaxed) {
            let level = if let Ok(buf) = buffer.try_lock() {
                compute_rms(&buf, 512)
            } else {
                0.0
            };
            let _ = app.emit_to("pill", "pill-level", level);
            tokio::time::sleep(std::time::Duration::from_millis(33)).await;
        }
        // Send a final zero level when stopping
        let _ = app.emit_to("pill", "pill-level", 0.0_f32);
    });
}

/// Compute normalized RMS from the last `window` samples of the buffer.
///
/// Returns 0.0-1.0 where typical speech is 0.3-0.8.
/// Uses a 10x multiplier: speech RMS is usually 0.01-0.1, so * 10 normalizes
/// to a usable range for the visualizer.
fn compute_rms(buf: &[f32], window: usize) -> f32 {
    if buf.is_empty() {
        return 0.0;
    }
    let n = buf.len().min(window);
    let tail = &buf[buf.len() - n..];
    let mean_sq: f32 = tail.iter().map(|&s| s * s).sum::<f32>() / n as f32;
    (mean_sq.sqrt() * 10.0).min(1.0)
}
