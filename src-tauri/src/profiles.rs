use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// A vocabulary profile that shapes transcription accuracy.
///
/// - `corrections`: Word-boundary find-and-replace dictionary applied after transcription.
///   Maps spoken approximations ("why section") to canonical forms ("W-section").
/// - `all_caps`: If true, all injected text is uppercased after corrections are applied.
#[derive(Clone, Serialize, Deserialize)]
pub struct Profile {
    pub corrections: HashMap<String, String>,
    pub all_caps: bool,
}

/// Returns a default empty profile.
pub fn default_profile() -> Profile {
    Profile {
        corrections: HashMap::new(),
        all_caps: false,
    }
}

/// Tauri managed state for the currently active profile.
///
/// Wrapped in a `Mutex` so it can be updated atomically.
pub struct ActiveProfile(pub std::sync::Mutex<Profile>);
