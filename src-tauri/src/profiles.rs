use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// A vocabulary profile that shapes whisper transcription accuracy for a domain.
///
/// - `initial_prompt`: Injected into whisper's FullParams to bias the model toward
///   domain-specific terminology. Must be non-empty for whisper to use it
///   (set_no_context must be false when initial_prompt is set — see transcribe.rs).
/// - `corrections`: Word-boundary find-and-replace dictionary applied after transcription.
///   Maps spoken approximations ("why section") to canonical forms ("W-section").
/// - `all_caps`: If true, all injected text is uppercased after corrections are applied.
#[derive(Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub initial_prompt: String,
    pub corrections: HashMap<String, String>,
    pub all_caps: bool,
}

/// Returns the hard-coded Structural Engineering profile.
///
/// initial_prompt: key engineering terms that bias whisper toward the domain vocabulary.
/// corrections: spoken approximations -> canonical engineering notation.
///
/// Both the prompt and corrections are locked decisions for v1 (see CONTEXT.md).
/// User additions are stored separately in settings.json and merged at runtime.
pub fn structural_engineering_profile() -> Profile {
    let mut corrections = HashMap::new();

    // Structural notation corrections (spoken form -> canonical form)
    corrections.insert("why section".to_string(), "W-section".to_string());
    corrections.insert("aci three eighteen".to_string(), "ACI 318".to_string());
    corrections.insert("aci 318".to_string(), "ACI 318".to_string());
    corrections.insert("pounds per square inch".to_string(), "PSI".to_string());
    corrections.insert("mpa".to_string(), "MPa".to_string());
    corrections.insert("ksi".to_string(), "ksi".to_string());
    corrections.insert("rebar".to_string(), "rebar".to_string());
    corrections.insert("i beam".to_string(), "I-beam".to_string());
    corrections.insert("w section".to_string(), "W-section".to_string());
    corrections.insert("w8 by 31".to_string(), "W8x31".to_string());
    corrections.insert("grade 60".to_string(), "Grade 60".to_string());
    corrections.insert("number 4 bar".to_string(), "#4 bar".to_string());
    corrections.insert("number 5 bar".to_string(), "#5 bar".to_string());

    Profile {
        id: "structural-engineering".to_string(),
        name: "Structural Engineering".to_string(),
        initial_prompt: "I-beam, W-section, W8x31, MPa, rebar, AISC, ACI 318, kips, PSI, \
            prestressed concrete, shear wall, moment frame, deflection, compressive strength, \
            tensile strength, grade 60 rebar"
            .to_string(),
        corrections,
        all_caps: false,
    }
}

/// Returns the hard-coded General profile.
///
/// Empty initial_prompt and no corrections — whisper runs with standard settings.
/// all_caps is false by default.
pub fn general_profile() -> Profile {
    Profile {
        id: "general".to_string(),
        name: "General".to_string(),
        initial_prompt: String::new(),
        corrections: HashMap::new(),
        all_caps: false,
    }
}

/// Returns all available built-in profiles.
///
/// v1 ships exactly two: Structural Engineering and General.
/// The order is intentional: Structural Engineering first (primary use case).
pub fn get_all_profiles() -> Vec<Profile> {
    vec![structural_engineering_profile(), general_profile()]
}

/// Tauri managed state for the currently active profile.
///
/// Wrapped in a `Mutex` so it can be swapped atomically when `set_active_profile`
/// is called. The `CorrectionsState` must also be rebuilt when the profile changes.
pub struct ActiveProfile(pub std::sync::Mutex<Profile>);
