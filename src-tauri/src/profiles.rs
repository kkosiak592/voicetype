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

    // Shear corrections (common Whisper misrecognition of "shear")
    corrections.insert("sheer".to_string(), "shear".to_string());
    corrections.insert("sheer wall".to_string(), "shear wall".to_string());
    corrections.insert("sheer force".to_string(), "shear force".to_string());
    corrections.insert("sheer stud".to_string(), "shear stud".to_string());
    corrections.insert("sheer connection".to_string(), "shear connection".to_string());
    corrections.insert("punching sheer".to_string(), "punching shear".to_string());

    // Shape and section corrections
    corrections.insert("why shape".to_string(), "W-shape".to_string());

    // Code and standard abbreviation corrections
    corrections.insert("a disc".to_string(), "AISC".to_string());
    corrections.insert("a see I".to_string(), "ACI".to_string());
    corrections.insert("a see E".to_string(), "ASCE".to_string());
    corrections.insert("Osh toe".to_string(), "AASHTO".to_string());
    corrections.insert("ash toe".to_string(), "AASHTO".to_string());
    corrections.insert("Alfred".to_string(), "LRFD".to_string());
    corrections.insert("E tabs".to_string(), "ETABS".to_string());
    corrections.insert("aisc 360".to_string(), "AISC 360".to_string());
    corrections.insert("aisc three sixty".to_string(), "AISC 360".to_string());
    corrections.insert("aisc 341".to_string(), "AISC 341".to_string());
    corrections.insert("aisc three forty one".to_string(), "AISC 341".to_string());
    corrections.insert("asce 7".to_string(), "ASCE 7".to_string());
    corrections.insert("asce seven".to_string(), "ASCE 7".to_string());
    corrections.insert("a disc 360".to_string(), "AISC 360".to_string());

    // Component and material corrections
    corrections.insert("rebirth".to_string(), "rebar".to_string());
    corrections.insert("re bar".to_string(), "rebar".to_string());
    corrections.insert("gust it".to_string(), "gusset".to_string());
    corrections.insert("stiffen her".to_string(), "stiffener".to_string());
    corrections.insert("fill it weld".to_string(), "fillet weld".to_string());
    corrections.insert("stir up".to_string(), "stirrup".to_string());
    corrections.insert("flex your".to_string(), "flexure".to_string());
    corrections.insert("lentil".to_string(), "lintel".to_string());

    // Named concept corrections
    corrections.insert("Oiler buckling".to_string(), "Euler buckling".to_string());
    corrections.insert("Moore's circle".to_string(), "Mohr's circle".to_string());
    corrections.insert("poison's ratio".to_string(), "Poisson's ratio".to_string());

    // Technique and method corrections
    corrections.insert("pre stressed".to_string(), "prestressed".to_string());
    corrections.insert("post tensioning".to_string(), "post-tensioning".to_string());

    // Rebar size corrections
    corrections.insert("number 3 bar".to_string(), "#3 bar".to_string());
    corrections.insert("number 6 bar".to_string(), "#6 bar".to_string());
    corrections.insert("number 7 bar".to_string(), "#7 bar".to_string());
    corrections.insert("number 8 bar".to_string(), "#8 bar".to_string());
    corrections.insert("number 9 bar".to_string(), "#9 bar".to_string());
    corrections.insert("number 10 bar".to_string(), "#10 bar".to_string());

    // Software name corrections
    corrections.insert("stood pro".to_string(), "STAAD Pro".to_string());
    corrections.insert("sap 2000".to_string(), "SAP2000".to_string());

    Profile {
        id: "structural-engineering".to_string(),
        name: "Structural Engineering".to_string(),
        initial_prompt: "I-beam, W-section, W-shape, W8x31, MPa, rebar, AISC, ACI 318, kips, PSI, \
    prestressed concrete, shear wall, moment frame, deflection, compressive strength, \
    tensile strength, grade 60 rebar, shear, gusset plate, stiffener, LRFD, ASD, \
    AASHTO, ASCE, post-tensioning, axial, flexure, buckling, diaphragm, splice, \
    ksi, DCR, ETABS, SAP2000, stirrup, lintel, fillet weld"
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
