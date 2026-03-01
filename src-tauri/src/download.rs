use futures_util::StreamExt;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tauri::ipc::Channel;
use tokio::io::AsyncWriteExt;

/// Events streamed to the frontend during a model download.
///
/// Tagged with `event` field and `data` content for easy frontend discrimination.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum DownloadEvent {
    Started { url: String, total_bytes: u64 },
    Progress { downloaded_bytes: u64, total_bytes: u64 },
    Finished,
    Error { message: String },
}

/// Returns the VoiceType models directory in APPDATA.
///
/// Duplicated from transcribe::models_dir() to avoid feature-gate coupling —
/// download.rs does not depend on whisper-rs.
fn models_dir() -> PathBuf {
    let appdata = std::env::var("APPDATA").expect("APPDATA environment variable not set");
    PathBuf::from(appdata).join("VoiceType").join("models")
}

/// Returns (filename, expected_sha256_hex, expected_size_bytes) for a known model_id,
/// or None if the model_id is not recognised.
fn model_info(model_id: &str) -> Option<(&'static str, &'static str, u64)> {
    match model_id {
        "large-v3-turbo" => Some((
            "ggml-large-v3-turbo-q5_0.bin",
            "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2",
            601_882_624,
        )),
        "small-en" => Some((
            "ggml-small.en-q5_1.bin",
            "bfdff4894dcb76bbf647d56263ea2a96645423f1669176f4844a1bf8e478ad30",
            199_229_440,
        )),
        _ => None,
    }
}

/// Constructs the HuggingFace resolve URL for a given model filename.
fn download_url(filename: &str) -> String {
    format!(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}",
        filename
    )
}

/// Download a whisper model file with streaming progress events and SHA256 validation.
///
/// Events are sent via the Tauri Channel so the frontend can display a progress bar
/// and handle errors without polling.
///
/// On checksum mismatch or any download error the temporary file is deleted, so a
/// subsequent launch correctly detects no model and re-shows the first-run setup flow.
#[tauri::command]
pub async fn download_model(
    model_id: String,
    on_event: Channel<DownloadEvent>,
) -> Result<(), String> {
    // Resolve model metadata
    let (filename, expected_sha256, expected_size_bytes) =
        model_info(&model_id).ok_or_else(|| format!("Unknown model id: {}", model_id))?;

    let url = download_url(filename);
    let dir = models_dir();
    let dest = dir.join(filename);
    let tmp_path = dest.with_extension("tmp");

    // Ensure models directory exists
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("Failed to create models directory: {}", e))?;

    // Notify frontend that download is starting
    let _ = on_event.send(DownloadEvent::Started {
        url: url.clone(),
        total_bytes: expected_size_bytes,
    });

    // Issue HTTP GET request
    let response = reqwest::Client::new()
        .get(&url)
        .send()
        .await
        .map_err(|e| {
            let msg = format!("HTTP request failed: {}", e);
            let _ = on_event.send(DownloadEvent::Error { message: msg.clone() });
            msg
        })?;

    // Use content-length from response if available; otherwise fall back to expected size
    let total_bytes = response.content_length().unwrap_or(expected_size_bytes);

    // Open temporary file for writing
    let mut file = tokio::fs::File::create(&tmp_path).await.map_err(|e| {
        let msg = format!("Failed to create temp file: {}", e);
        let _ = on_event.send(DownloadEvent::Error { message: msg.clone() });
        msg
    })?;

    let mut hasher = Sha256::new();
    let mut downloaded_bytes: u64 = 0;
    let mut stream = response.bytes_stream();

    // Stream chunks, write to disk, feed hasher, emit progress events
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| {
            let msg = format!("Download stream error: {}", e);
            let _ = on_event.send(DownloadEvent::Error { message: msg.clone() });
            // Best-effort cleanup of partial temp file
            let tmp = tmp_path.clone();
            tokio::spawn(async move {
                let _ = tokio::fs::remove_file(tmp).await;
            });
            msg
        })?;

        file.write_all(&chunk).await.map_err(|e| {
            let msg = format!("Failed to write chunk to temp file: {}", e);
            let _ = on_event.send(DownloadEvent::Error { message: msg.clone() });
            let tmp = tmp_path.clone();
            tokio::spawn(async move {
                let _ = tokio::fs::remove_file(tmp).await;
            });
            msg
        })?;

        hasher.update(&chunk);
        downloaded_bytes += chunk.len() as u64;

        let _ = on_event.send(DownloadEvent::Progress {
            downloaded_bytes,
            total_bytes,
        });
    }

    // Flush and close the file before renaming
    file.flush().await.map_err(|e| {
        let msg = format!("Failed to flush temp file: {}", e);
        let _ = on_event.send(DownloadEvent::Error { message: msg.clone() });
        msg
    })?;
    drop(file);

    // Validate SHA256 checksum
    let actual_hex = format!("{:x}", hasher.finalize());
    if actual_hex != expected_sha256 {
        // Delete corrupt temp file so next launch re-shows setup flow
        let _ = tokio::fs::remove_file(&tmp_path).await;
        let msg = format!(
            "Checksum mismatch: expected {}, got {}",
            expected_sha256, actual_hex
        );
        let _ = on_event.send(DownloadEvent::Error { message: msg.clone() });
        return Err(msg);
    }

    // Atomically move temp file to final destination
    tokio::fs::rename(&tmp_path, &dest).await.map_err(|e| {
        let msg = format!("Failed to move model file into place: {}", e);
        let _ = on_event.send(DownloadEvent::Error { message: msg.clone() });
        msg
    })?;

    log::info!(
        "Model '{}' downloaded and verified successfully ({})",
        model_id,
        dest.display()
    );

    let _ = on_event.send(DownloadEvent::Finished);
    Ok(())
}
