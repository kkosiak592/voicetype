use serde::{Deserialize, Serialize};
use tauri::Manager;

/// A single transcription history entry.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub text: String,
    pub timestamp_ms: u64,
    pub engine: String,
    /// Raw (pre-correction) text from the transcription pipeline.
    /// Only present when at least one correction was applied — None means the
    /// formatted text is identical to the raw transcription (or entry predates this feature).
    /// `#[serde(default)]` ensures existing history.json files without this field
    /// deserialize cleanly as None.
    #[serde(default)]
    pub raw_text: Option<String>,
}

/// Mutex-backed managed state for transcription history.
pub struct HistoryState(pub std::sync::Mutex<Vec<HistoryEntry>>);

fn history_path(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_data_dir().ok().map(|d| d.join("history.json"))
}

/// Load history entries from disk. Returns empty vec if file missing or unparseable.
pub fn load_history(app: &tauri::AppHandle) -> Vec<HistoryEntry> {
    history_path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_history(app: &tauri::AppHandle, entries: &[HistoryEntry]) {
    if let Some(path) = history_path(app) {
        let _ = std::fs::write(path, serde_json::to_string_pretty(entries).unwrap_or_default());
    }
}

/// Append a new transcription entry to history.
///
/// Inserts newest-first, caps at 50 entries, and persists to disk.
/// Called from pipeline.rs after successful injection.
///
/// `raw_text`: the pre-correction text. Pass `Some(raw)` when corrections were applied
/// (raw != formatted). Pass `None` when no corrections changed the text.
pub fn append_history(app: &tauri::AppHandle, text: &str, engine: &str, raw_text: Option<&str>) {
    let entry = HistoryEntry {
        text: text.to_string(),
        timestamp_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        engine: engine.to_string(),
        raw_text: raw_text.map(|s| s.to_string()),
    };
    let state = app.state::<HistoryState>();
    let mut guard = state.0.lock().unwrap_or_else(|e: std::sync::PoisonError<_>| e.into_inner());
    guard.insert(0, entry); // newest first
    guard.truncate(50); // cap at 50 per locked decision
    save_history(app, &guard);
    
    use tauri::Emitter;
    app.emit("history-updated", ()).ok();
}
