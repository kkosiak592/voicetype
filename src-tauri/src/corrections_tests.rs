#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::corrections::CorrectionsEngine;

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
}
