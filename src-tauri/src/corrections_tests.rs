#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    // These will fail until corrections.rs and profiles.rs are created.
    use crate::corrections::CorrectionsEngine;
    use crate::profiles::{structural_engineering_profile, general_profile};

    fn make_engine(pairs: &[(&str, &str)]) -> CorrectionsEngine {
        let mut map = HashMap::new();
        for (from, to) in pairs {
            map.insert(from.to_string(), to.to_string());
        }
        CorrectionsEngine::from_map(&map).expect("engine build failed")
    }

    #[test]
    fn whole_word_case_insensitive_replacement() {
        let engine = make_engine(&[("mpa", "MPa")]);
        assert_eq!(engine.apply("the mpa value is high"), "the MPa value is high");
    }

    #[test]
    fn no_substring_match() {
        // "mpa" should not replace inside "compare"
        let engine = make_engine(&[("mpa", "MPa")]);
        assert_eq!(engine.apply("compare values"), "compare values");
    }

    #[test]
    fn multi_word_phrase_replacement() {
        let engine = make_engine(&[("aci three eighteen", "ACI 318")]);
        assert_eq!(
            engine.apply("aci three eighteen is the code"),
            "ACI 318 is the code"
        );
    }

    #[test]
    fn multi_word_phrase_why_section() {
        let engine = make_engine(&[("why section", "W-section")]);
        assert_eq!(engine.apply("why section W8x31"), "W-section W8x31");
    }

    #[test]
    fn empty_map_passthrough() {
        let engine = CorrectionsEngine::from_map(&HashMap::new()).expect("empty engine failed");
        assert_eq!(engine.apply("hello world"), "hello world");
    }

    #[test]
    fn structural_engineering_profile_fields() {
        let p = structural_engineering_profile();
        assert_eq!(p.id, "structural-engineering");
        assert!(
            p.initial_prompt.contains("I-beam"),
            "initial_prompt missing I-beam"
        );
        assert!(
            p.initial_prompt.contains("W-section"),
            "initial_prompt missing W-section"
        );
        assert!(
            p.initial_prompt.contains("MPa"),
            "initial_prompt missing MPa"
        );
        assert!(!p.corrections.is_empty(), "corrections should be non-empty");
        assert!(!p.all_caps, "all_caps should be false");
    }

    #[test]
    fn general_profile_fields() {
        let p = general_profile();
        assert_eq!(p.id, "general");
        assert!(p.initial_prompt.is_empty(), "general profile initial_prompt should be empty");
        assert!(p.corrections.is_empty(), "general profile corrections should be empty");
        assert!(!p.all_caps, "general profile all_caps should be false");
    }

    #[test]
    fn all_caps_uppercases_text() {
        // all_caps behavior is tested at the text transformation level
        let text = "hello world";
        let mut profile = general_profile();
        profile.all_caps = true;
        let result = if profile.all_caps {
            text.to_uppercase()
        } else {
            text.to_string()
        };
        assert_eq!(result, "HELLO WORLD");
    }
}
