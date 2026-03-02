use futures_util::StreamExt;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tauri::ipc::Channel;
use tokio::io::AsyncWriteExt;

/// Parakeet TDT int8 ONNX files from HuggingFace repo istupakov/parakeet-tdt-0.6b-v2-onnx.
///
/// Each entry is (remote_filename, local_filename, expected_size_bytes).
/// Remote names are the HuggingFace filenames; local names match what parakeet-rs expects
/// (it looks for "encoder-model.onnx" and "decoder_joint-model.onnx", not the int8 variants).
const PARAKEET_FILES: &[(&str, &str, u64)] = &[
    ("encoder-model.int8.onnx", "encoder-model.onnx", 652_184_014),
    ("decoder_joint-model.int8.onnx", "decoder_joint-model.onnx", 8_998_286),
    ("nemo128.onnx", "nemo128.onnx", 139_764),
    ("vocab.txt", "vocab.txt", 9_384),
    ("config.json", "config.json", 97),
];

/// Events streamed to the frontend during a model download.
///
/// Tagged with `event` field and `data` content for easy frontend discrimination.
///
/// NOTE: `rename_all = "camelCase"` on the enum container renames variant discriminants
/// (Started->"started", Progress->"progress") but NOT the field names inside struct variants.
/// Field names require explicit per-field `#[serde(rename)]` to match the camelCase keys
/// the frontend reads from `msg.data`.
#[derive(Clone, Serialize)]
#[serde(tag = "event", content = "data")]
pub enum DownloadEvent {
    #[serde(rename = "started")]
    Started {
        url: String,
        #[serde(rename = "totalBytes")]
        total_bytes: u64,
    },
    #[serde(rename = "progress")]
    Progress {
        #[serde(rename = "downloadedBytes")]
        downloaded_bytes: u64,
        #[serde(rename = "totalBytes")]
        total_bytes: u64,
    },
    #[serde(rename = "finished")]
    Finished,
    #[serde(rename = "error")]
    Error {
        message: String,
    },
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

// ── Parakeet TDT model download ────────────────────────────────────────────

/// Parakeet TDT fp32 ONNX files from HuggingFace repo istupakov/parakeet-tdt-0.6b-v2-onnx.
///
/// fp32 uses ONNX external data format: encoder-model.onnx is a small header (~42MB),
/// encoder-model.onnx.data contains the actual weights (~2.44GB). Both must be co-located.
/// Remote filenames match local filenames (no renaming needed — unlike int8 which has .int8. prefix).
const PARAKEET_FP32_FILES: &[(&str, &str, u64)] = &[
    ("encoder-model.onnx", "encoder-model.onnx", 41_800_000),
    ("encoder-model.onnx.data", "encoder-model.onnx.data", 2_440_000_000),
    ("decoder_joint-model.onnx", "decoder_joint-model.onnx", 35_800_000),
    ("nemo128.onnx", "nemo128.onnx", 139_764),
    ("vocab.txt", "vocab.txt", 9_384),
    ("config.json", "config.json", 97),
];

/// Returns the directory where the Parakeet TDT model files are stored.
pub fn parakeet_model_dir() -> PathBuf {
    models_dir().join("parakeet-tdt-v2")
}

/// Returns the directory where the Parakeet TDT fp32 model files are stored.
pub fn parakeet_fp32_model_dir() -> PathBuf {
    models_dir().join("parakeet-tdt-v2-fp32")
}

/// Returns true if the Parakeet TDT model appears to have been fully downloaded.
///
/// Checks for the encoder ONNX file only — it is the largest file and the last
/// successfully atomic-renamed file in a complete download sequence.
pub fn parakeet_model_exists() -> bool {
    parakeet_model_dir()
        .join("encoder-model.onnx")
        .exists()
}

/// Returns true if the Parakeet TDT fp32 model appears to have been fully downloaded.
///
/// Checks for encoder-model.onnx (the ONNX header file) — both the header and
/// the .data weights file must be present for the model to load correctly.
pub fn parakeet_fp32_model_exists() -> bool {
    parakeet_fp32_model_dir()
        .join("encoder-model.onnx")
        .exists()
}

/// Constructs the HuggingFace resolve URL for a Parakeet int8 ONNX file.
fn parakeet_download_url(filename: &str) -> String {
    format!(
        "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v2-onnx/resolve/main/{}",
        filename
    )
}

/// Downloads all 5 Parakeet TDT int8 ONNX files to models/parakeet-tdt-v2/ with streaming
/// progress events.
///
/// Events use the same `DownloadEvent` enum as `download_model` so the frontend can reuse
/// its progress-bar logic. Progress is cumulative across all files (single progress bar).
///
/// On any file error the entire parakeet-tdt-v2/ directory is removed so a subsequent launch
/// re-shows the download prompt. Each file is written atomically via .tmp-then-rename.
#[tauri::command]
pub async fn download_parakeet_model(on_event: Channel<DownloadEvent>) -> Result<(), String> {
    let dest_dir = parakeet_model_dir();

    // Ensure destination directory exists
    tokio::fs::create_dir_all(&dest_dir)
        .await
        .map_err(|e| format!("Failed to create parakeet model directory: {}", e))?;

    // Total expected bytes across all files (used for progress denominator)
    let total_bytes: u64 = PARAKEET_FILES.iter().map(|(_, _, size)| size).sum();

    let _ = on_event.send(DownloadEvent::Started {
        url: "parakeet-tdt-v2 (5 files)".to_string(),
        total_bytes,
    });

    let client = reqwest::Client::new();
    let mut cumulative_downloaded: u64 = 0;

    for (remote_name, local_name, expected_size) in PARAKEET_FILES {
        let url = parakeet_download_url(remote_name);
        let dest = dest_dir.join(local_name);
        let tmp_path = dest.with_extension("tmp");

        log::info!("Downloading Parakeet file: {} -> {} ({})", remote_name, local_name, url);

        let response = client.get(&url).send().await.map_err(|e| {
            let msg = format!("HTTP request failed for {}: {}", remote_name, e);
            let _ = on_event.send(DownloadEvent::Error { message: msg.clone() });
            // Best-effort cleanup of entire model directory on error
            let dir = dest_dir.clone();
            tokio::spawn(async move {
                let _ = tokio::fs::remove_dir_all(dir).await;
            });
            msg
        })?;

        // Use content-length from response if available; otherwise fall back to expected size
        let file_total = response.content_length().unwrap_or(*expected_size);
        let _ = file_total; // used implicitly via cumulative progress

        let mut file = tokio::fs::File::create(&tmp_path).await.map_err(|e| {
            let msg = format!("Failed to create temp file for {}: {}", local_name, e);
            let _ = on_event.send(DownloadEvent::Error { message: msg.clone() });
            let dir = dest_dir.clone();
            tokio::spawn(async move {
                let _ = tokio::fs::remove_dir_all(dir).await;
            });
            msg
        })?;

        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| {
                let msg = format!("Download stream error for {}: {}", remote_name, e);
                let _ = on_event.send(DownloadEvent::Error { message: msg.clone() });
                let tmp = tmp_path.clone();
                let dir = dest_dir.clone();
                tokio::spawn(async move {
                    let _ = tokio::fs::remove_file(tmp).await;
                    let _ = tokio::fs::remove_dir_all(dir).await;
                });
                msg
            })?;

            file.write_all(&chunk).await.map_err(|e| {
                let msg = format!("Failed to write chunk for {}: {}", local_name, e);
                let _ = on_event.send(DownloadEvent::Error { message: msg.clone() });
                let tmp = tmp_path.clone();
                let dir = dest_dir.clone();
                tokio::spawn(async move {
                    let _ = tokio::fs::remove_file(tmp).await;
                    let _ = tokio::fs::remove_dir_all(dir).await;
                });
                msg
            })?;

            cumulative_downloaded += chunk.len() as u64;

            let _ = on_event.send(DownloadEvent::Progress {
                downloaded_bytes: cumulative_downloaded,
                total_bytes,
            });
        }

        // Flush and close before rename
        file.flush().await.map_err(|e| {
            let msg = format!("Failed to flush temp file for {}: {}", local_name, e);
            let _ = on_event.send(DownloadEvent::Error { message: msg.clone() });
            msg
        })?;
        drop(file);

        // Atomically move temp file to final destination
        tokio::fs::rename(&tmp_path, &dest).await.map_err(|e| {
            let msg = format!("Failed to rename temp file for {}: {}", local_name, e);
            let _ = on_event.send(DownloadEvent::Error { message: msg.clone() });
            msg
        })?;

        log::info!(
            "Parakeet file downloaded: {} ({} bytes cumulative)",
            local_name,
            cumulative_downloaded
        );
    }

    log::info!(
        "Parakeet TDT model download complete ({} bytes, {})",
        cumulative_downloaded,
        dest_dir.display()
    );

    let _ = on_event.send(DownloadEvent::Finished);
    Ok(())
}

/// Downloads all 6 Parakeet TDT fp32 ONNX files to models/parakeet-tdt-v2-fp32/ with streaming
/// progress events.
///
/// The fp32 model uses ONNX external data format — encoder-model.onnx is a small header file
/// and encoder-model.onnx.data holds the actual weights (~2.44GB). Both must be co-located
/// in the same directory for ONNX Runtime to load the model.
///
/// Progress is cumulative across all 6 files (single progress bar).
/// On any file error the entire parakeet-tdt-v2-fp32/ directory is removed.
#[tauri::command]
pub async fn download_parakeet_fp32_model(on_event: Channel<DownloadEvent>) -> Result<(), String> {
    let dest_dir = parakeet_fp32_model_dir();

    // Ensure destination directory exists
    tokio::fs::create_dir_all(&dest_dir)
        .await
        .map_err(|e| format!("Failed to create parakeet fp32 model directory: {}", e))?;

    // Total expected bytes across all files (used for progress denominator)
    let total_bytes: u64 = PARAKEET_FP32_FILES.iter().map(|(_, _, size)| size).sum();

    let _ = on_event.send(DownloadEvent::Started {
        url: "parakeet-tdt-v2-fp32 (6 files)".to_string(),
        total_bytes,
    });

    let client = reqwest::Client::new();
    let mut cumulative_downloaded: u64 = 0;

    for (remote_name, local_name, expected_size) in PARAKEET_FP32_FILES {
        let url = parakeet_download_url(remote_name);
        let dest = dest_dir.join(local_name);
        let tmp_path = dest.with_extension("tmp");

        log::info!("Downloading Parakeet fp32 file: {} -> {} ({})", remote_name, local_name, url);

        let response = client.get(&url).send().await.map_err(|e| {
            let msg = format!("HTTP request failed for {}: {}", remote_name, e);
            let _ = on_event.send(DownloadEvent::Error { message: msg.clone() });
            let dir = dest_dir.clone();
            tokio::spawn(async move {
                let _ = tokio::fs::remove_dir_all(dir).await;
            });
            msg
        })?;

        // Use content-length from response if available; otherwise fall back to expected size
        let file_total = response.content_length().unwrap_or(*expected_size);
        let _ = file_total; // used implicitly via cumulative progress

        let mut file = tokio::fs::File::create(&tmp_path).await.map_err(|e| {
            let msg = format!("Failed to create temp file for {}: {}", local_name, e);
            let _ = on_event.send(DownloadEvent::Error { message: msg.clone() });
            let dir = dest_dir.clone();
            tokio::spawn(async move {
                let _ = tokio::fs::remove_dir_all(dir).await;
            });
            msg
        })?;

        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| {
                let msg = format!("Download stream error for {}: {}", remote_name, e);
                let _ = on_event.send(DownloadEvent::Error { message: msg.clone() });
                let tmp = tmp_path.clone();
                let dir = dest_dir.clone();
                tokio::spawn(async move {
                    let _ = tokio::fs::remove_file(tmp).await;
                    let _ = tokio::fs::remove_dir_all(dir).await;
                });
                msg
            })?;

            file.write_all(&chunk).await.map_err(|e| {
                let msg = format!("Failed to write chunk for {}: {}", local_name, e);
                let _ = on_event.send(DownloadEvent::Error { message: msg.clone() });
                let tmp = tmp_path.clone();
                let dir = dest_dir.clone();
                tokio::spawn(async move {
                    let _ = tokio::fs::remove_file(tmp).await;
                    let _ = tokio::fs::remove_dir_all(dir).await;
                });
                msg
            })?;

            cumulative_downloaded += chunk.len() as u64;

            let _ = on_event.send(DownloadEvent::Progress {
                downloaded_bytes: cumulative_downloaded,
                total_bytes,
            });
        }

        // Flush and close before rename
        file.flush().await.map_err(|e| {
            let msg = format!("Failed to flush temp file for {}: {}", local_name, e);
            let _ = on_event.send(DownloadEvent::Error { message: msg.clone() });
            msg
        })?;
        drop(file);

        // Atomically move temp file to final destination
        tokio::fs::rename(&tmp_path, &dest).await.map_err(|e| {
            let msg = format!("Failed to rename temp file for {}: {}", local_name, e);
            let _ = on_event.send(DownloadEvent::Error { message: msg.clone() });
            msg
        })?;

        log::info!(
            "Parakeet fp32 file downloaded: {} ({} bytes cumulative)",
            local_name,
            cumulative_downloaded
        );
    }

    log::info!(
        "Parakeet TDT fp32 model download complete ({} bytes, {})",
        cumulative_downloaded,
        dest_dir.display()
    );

    let _ = on_event.send(DownloadEvent::Finished);
    Ok(())
}
