# Quick Task 47: Remove stale Parakeet vocabulary prompting warning from model page - Context

**Gathered:** 2026-03-07
**Status:** Ready for planning

<domain>
## Task Boundary

Remove stale Parakeet vocabulary prompting warning from model page. The model settings page still shows a warning message about vocabulary prompting not being supported, but vocabulary prompting was already fully removed (quick task #38, commit 6c3616b). This text is now stale and confusing.

</domain>

<decisions>
## Implementation Decisions

### Removal scope
- Full cleanup: Remove warning text/JSX AND any dead conditional blocks, state, or props that only existed to serve this warning

### Residual references
- Full sweep: Search the entire codebase for any remaining vocabulary_prompting, initial_prompt, or related stale references beyond ModelSection.tsx

</decisions>

<specifics>
## Specific Ideas

- Known location: `src/components/sections/ModelSection.tsx:235`
- Vocabulary prompting was removed in quick task #38 (commit 6c3616b)
- Search terms for sweep: `vocabulary_prompting`, `initial_prompt`, `vocabulary prompting`, `doesn't support vocabulary`

</specifics>
