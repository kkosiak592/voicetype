use serde::{Deserialize, Serialize};
use tauri::Manager;

/// The threshold of occurrences before a correction is auto-promoted to the dictionary.
const PROMOTE_THRESHOLD: u32 = 3;

/// A single from->to correction pair with occurrence count.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CorrectionEntry {
    pub from: String,
    pub to: String,
    pub count: u32,
}

/// A promoted correction returned to the frontend so it can show a notification.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PromotedCorrection {
    pub from: String,
    pub to: String,
}

/// Vec-backed correction log with load/save/record/remove methods.
pub struct CorrectionLog {
    entries: Vec<CorrectionEntry>,
}

impl CorrectionLog {
    fn new() -> Self {
        CorrectionLog { entries: Vec::new() }
    }

    /// Load correction log from disk. Returns empty log if file missing or corrupt.
    pub fn load(app: &tauri::AppHandle) -> Self {
        let path = match correction_log_path(app) {
            Some(p) => p,
            None => return Self::new(),
        };
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => return Self::new(),
        };
        let entries: Vec<CorrectionEntry> = serde_json::from_str(&text).unwrap_or_default();
        CorrectionLog { entries }
    }

    /// Write correction log to disk.
    pub fn save(&self, app: &tauri::AppHandle) {
        if let Some(path) = correction_log_path(app) {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let json = serde_json::to_string_pretty(&self.entries).unwrap_or_default();
            let _ = std::fs::write(path, json);
        }
    }

    /// Record a from->to correction. Increments count for existing entry or inserts new one.
    ///
    /// Returns `Some(PromotedCorrection)` if this record caused the count to reach
    /// `PROMOTE_THRESHOLD` for the first time. Returns `None` if the count was already
    /// at or above the threshold (already promoted) or if threshold not yet reached.
    pub fn record(&mut self, from: String, to: String) -> Option<PromotedCorrection> {
        // Case-insensitive match on `from`, exact match on `to`.
        let existing = self.entries.iter_mut().find(|e| {
            e.from.to_lowercase() == from.to_lowercase() && e.to == to
        });
        match existing {
            Some(entry) => {
                let before = entry.count;
                entry.count += 1;
                // Promote only when count crosses the threshold (was below, now at/above).
                if before < PROMOTE_THRESHOLD && entry.count >= PROMOTE_THRESHOLD {
                    Some(PromotedCorrection {
                        from: entry.from.clone(),
                        to: entry.to.clone(),
                    })
                } else {
                    None
                }
            }
            None => {
                self.entries.push(CorrectionEntry {
                    from: from.clone(),
                    to: to.clone(),
                    count: 1,
                });
                // Threshold is 3; count 1 never triggers promotion.
                None
            }
        }
    }

    /// Remove a from->to entry (used when the user undoes an auto-promotion).
    pub fn remove(&mut self, from: &str, to: &str) {
        self.entries.retain(|e| {
            !(e.from.to_lowercase() == from.to_lowercase() && e.to == to)
        });
    }
}

/// Tauri managed-state wrapper for the correction log.
pub struct CorrectionLogState(pub std::sync::Mutex<CorrectionLog>);

/// Load correction log from disk and wrap in managed state.
pub fn load_correction_log(app: &tauri::AppHandle) -> CorrectionLogState {
    CorrectionLogState(std::sync::Mutex::new(CorrectionLog::load(app)))
}

fn correction_log_path(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("corrections_log.json"))
}
