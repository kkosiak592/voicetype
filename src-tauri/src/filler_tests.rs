#[cfg(test)]
mod tests {
    use crate::filler::remove_fillers;

    #[test]
    fn removes_um_at_start() {
        assert_eq!(remove_fillers("um I think so"), "I think so");
    }

    #[test]
    fn removes_uh_at_start() {
        assert_eq!(remove_fillers("uh what was that"), "what was that");
    }

    #[test]
    fn removes_uh_huh_multi_word() {
        assert_eq!(remove_fillers("uh huh that works"), "that works");
    }

    #[test]
    fn removes_hmm_at_start() {
        assert_eq!(remove_fillers("hmm let me think"), "let me think");
    }

    #[test]
    fn removes_er_at_start() {
        assert_eq!(remove_fillers("er the thing is"), "the thing is");
    }

    #[test]
    fn removes_ah_at_start() {
        assert_eq!(remove_fillers("ah I see"), "I see");
    }

    #[test]
    fn case_insensitive_um() {
        assert_eq!(remove_fillers("Um I think"), "I think");
    }

    #[test]
    fn removes_filler_mid_sentence() {
        assert_eq!(remove_fillers("I um think so"), "I think so");
    }

    #[test]
    fn removes_multiple_fillers() {
        assert_eq!(remove_fillers("um uh I think"), "I think");
    }

    #[test]
    fn collapses_double_spaces() {
        // "I um think" -> "I think" (not "I  think")
        assert_eq!(remove_fillers("I um think"), "I think");
    }

    #[test]
    fn trims_leading_trailing_whitespace() {
        assert_eq!(remove_fillers("  um  I see  "), "I see");
    }

    #[test]
    fn preserves_umbrella() {
        assert_eq!(remove_fillers("umbrella"), "umbrella");
    }

    #[test]
    fn preserves_hummingbird() {
        assert_eq!(remove_fillers("hummingbird"), "hummingbird");
    }

    #[test]
    fn preserves_errand() {
        assert_eq!(remove_fillers("errand"), "errand");
    }

    #[test]
    fn empty_result_after_all_fillers() {
        assert_eq!(remove_fillers("um uh hmm"), "");
    }

    #[test]
    fn empty_input_returns_empty() {
        assert_eq!(remove_fillers(""), "");
    }

    #[test]
    fn no_fillers_unchanged() {
        assert_eq!(remove_fillers("hello world"), "hello world");
    }
}
