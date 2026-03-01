use std::collections::HashMap;
use regex::Regex;

/// A compiled corrections rule: a word-boundary-anchored regex pattern paired
/// with its replacement string.
///
/// Patterns are compiled as `(?i)\b{escaped_from}\b` — case-insensitive,
/// whole-word matching. Multi-word phrases work because `\b` anchors are placed
/// only at the outer edges of the pattern.
struct Rule {
    pattern: Regex,
    replacement: String,
}

/// HashMap-backed corrections engine with regex word-boundary matching.
///
/// Built once from a dictionary of `{from: to}` pairs. Each `from` key becomes
/// a compiled regex rule. The engine applies all rules to a text string in order.
///
/// Ordering: rules are applied in arbitrary HashMap iteration order. This is
/// acceptable for v1 because the dictionary is designed such that rules don't
/// conflict. Multi-word phrases take priority by virtue of being more specific
/// regexes — `\baci three eighteen\b` won't accidentally match inside a
/// replacement result because replacements happen sequentially.
pub struct CorrectionsEngine {
    rules: Vec<Rule>,
}

impl CorrectionsEngine {
    /// Build a CorrectionsEngine from a dictionary of `{from -> to}` pairs.
    ///
    /// Each `from` key is compiled into a regex: `(?i)\b{escaped_from}\b`.
    /// `regex::escape` is used to safely escape any regex metacharacters in
    /// the from-key before embedding it in the pattern.
    ///
    /// Returns an error if any regex fails to compile (should not happen with
    /// escaped inputs, but surface the error rather than panic).
    pub fn from_map(map: &HashMap<String, String>) -> Result<Self, String> {
        let mut rules = Vec::with_capacity(map.len());

        for (from, to) in map {
            let escaped = regex::escape(from);
            let pattern_str = format!(r"(?i)\b{}\b", escaped);
            let pattern = Regex::new(&pattern_str)
                .map_err(|e| format!("Failed to compile corrections regex for '{}': {}", from, e))?;
            rules.push(Rule {
                pattern,
                replacement: to.clone(),
            });
        }

        Ok(CorrectionsEngine { rules })
    }

    /// Apply all correction rules to `text` and return the result.
    ///
    /// Each rule is applied via `Regex::replace_all()` which replaces every
    /// non-overlapping match in the string. Rules are applied sequentially —
    /// a replacement from one rule can be matched by a later rule if they overlap,
    /// but the dictionary is designed to avoid such conflicts.
    pub fn apply(&self, text: &str) -> String {
        let mut result = text.to_string();
        for rule in &self.rules {
            let replaced = rule.pattern.replace_all(&result, rule.replacement.as_str());
            result = replaced.into_owned();
        }
        result
    }
}

/// Tauri managed state wrapper for the corrections engine.
///
/// Wrapped in a `Mutex` so it can be replaced atomically when the active
/// profile changes or the user saves an updated corrections dictionary.
pub struct CorrectionsState(pub std::sync::Mutex<CorrectionsEngine>);
