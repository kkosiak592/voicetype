# Quick Task 45: Learn from user corrections to auto-improve dictionary - Context

**Gathered:** 2026-03-05
**Status:** Ready for planning

<domain>
## Task Boundary

Learn from user corrections to auto-improve dictionary. When users manually correct transcription output, track those corrections and automatically grow the corrections dictionary over time.

</domain>

<decisions>
## Implementation Decisions

### Correction Detection
- Add a correction UI panel where the user can review the last transcription and submit corrections explicitly

### Auto-Add Threshold
- 3 repetitions required before auto-adding to dictionary
- Show a toast notification when a correction is auto-added, letting the user undo

### Storage & Persistence
- Separate JSON file per profile (`corrections_log.json`) alongside existing profile config
- Clean separation from profile settings, easy to inspect and reset

### Claude's Discretion
- Diff algorithm details for extracting word-level corrections from user edits
- UI placement and styling of the correction panel

</decisions>

<specifics>
## Specific Ideas

- Existing corrections system uses `CorrectionsEngine` with `HashMap<String, String>` from→to pairs
- `DictionaryEditor` component handles manual dictionary editing
- Per-profile config already exists — log file lives next to it
- Dragon NaturallySpeaking-style learning is the inspiration

</specifics>
