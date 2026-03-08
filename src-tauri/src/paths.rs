use std::path::PathBuf;

/// Returns the VoiceType models directory in APPDATA.
///
/// Shared by download.rs and transcribe.rs to avoid duplication.
/// Not feature-gated so both modules can use it regardless of compiled features.
pub fn models_dir() -> Result<PathBuf, String> {
    let appdata = std::env::var("APPDATA")
        .map_err(|_| "APPDATA environment variable not set".to_string())?;
    Ok(PathBuf::from(appdata).join("VoiceType").join("models"))
}
