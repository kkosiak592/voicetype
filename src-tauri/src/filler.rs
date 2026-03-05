use regex::Regex;
use std::sync::OnceLock;

/// Compiled filler word removal patterns.
///
/// Multi-word fillers (e.g. "uh huh") are listed first so they are matched
/// and removed before the single-word variants ("uh") have a chance to match
/// partial substrings within the multi-word phrase.
struct FillerPatterns {
    patterns: Vec<Regex>,
}

impl FillerPatterns {
    fn build() -> Self {
        // Order matters: multi-word before single-word to prevent partial matches.
        let fillers = [
            "uh huh",
            "um",
            "uh",
            "hmm",
            "er",
            "ah",
        ];

        let patterns = fillers
            .iter()
            .map(|filler| {
                let escaped = regex::escape(filler);
                // Case-insensitive whole-word matching — same pattern as corrections engine.
                Regex::new(&format!(r"(?i)\b{}\b", escaped))
                    .expect("filler regex failed to compile")
            })
            .collect();

        FillerPatterns { patterns }
    }
}

/// Global compiled filler patterns — compiled once on first call.
static FILLER_PATTERNS: OnceLock<FillerPatterns> = OnceLock::new();

fn get_patterns() -> &'static FillerPatterns {
    FILLER_PATTERNS.get_or_init(FillerPatterns::build)
}

/// Remove filler words (hesitation sounds) from transcribed text.
///
/// Strips: um, uh, uh huh, hmm, er, ah — whole-word only, case-insensitive.
/// Multi-word fillers ("uh huh") are removed before single-word ones.
/// After removal, whitespace is normalised (collapsed + trimmed).
///
/// Non-filler words containing filler substrings are preserved:
///   "umbrella" → "umbrella", "hummingbird" → "hummingbird", "errand" → "errand"
pub fn remove_fillers(text: &str) -> String {
    let patterns = get_patterns();

    let mut result = text.to_string();
    for pattern in &patterns.patterns {
        result = pattern.replace_all(&result, "").to_string();
    }

    // Normalise whitespace: collapse multiple spaces, trim leading/trailing.
    result
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
