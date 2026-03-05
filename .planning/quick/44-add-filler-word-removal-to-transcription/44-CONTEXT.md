# Quick Task 44: Add filler word removal to transcription output - Context

**Gathered:** 2026-03-05
**Status:** Ready for planning

<domain>
## Task Boundary

Add filler word removal to transcription output. Transcription output includes filler words like "um", "uh", "like", "you know" which clutter the final text and require manual cleanup. Simple regex/dictionary approach — maintain a list of common filler words/phrases and strip them from the transcription output as a post-processing step before text injection. Should be toggleable in settings.

</domain>

<decisions>
## Implementation Decisions

### Filler Word Scope
- Conservative list only: hesitation sounds (um, uh, uh huh, hmm, er, ah)
- No discourse markers (like, you know, basically, etc.) — too high risk of false positives

### Customizable List
- Hardcoded list only — no UI for editing filler words
- Users can use the existing dictionary/corrections system for custom removals if needed

### Pipeline Placement
- Filler removal runs BEFORE corrections in the pipeline
- Strip fillers first, then apply word corrections
- Corrections won't need to account for filler words in their patterns

### Claude's Discretion
- None — all areas discussed

</decisions>

<specifics>
## Specific Ideas

- Toggle in settings UI (General section, near ALL CAPS toggle)
- Setting key: `filler_removal` (bool, default false)
- Regex-based whole-word matching similar to corrections engine
- Handle edge cases: standalone "like" vs "like" in phrases, capitalization variants

</specifics>
