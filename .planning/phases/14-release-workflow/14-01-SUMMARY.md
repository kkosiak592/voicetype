---
phase: 14-release-workflow
plan: "01"
subsystem: infra
tags: [release, changelog, runbook, semver, git-tags, keep-a-changelog]

# Dependency graph
requires:
  - phase: 13-ci-cd-pipeline
    provides: GitHub Actions release workflow triggered by v* tag push
provides:
  - RELEASING.md release runbook covering full version bump to push flow
  - CHANGELOG.md with Keep a Changelog format and v0.1.0 baseline entry
affects: [future releases, release automation skill]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Keep a Changelog format (Added/Changed/Fixed/Removed categories)
    - Annotated git tags (git tag -a) for semantic versioning
    - Conventional commit format for release commits (release: vX.Y.Z)
    - git push --follow-tags to push commit and tag in one step

key-files:
  created:
    - RELEASING.md
    - CHANGELOG.md
  modified: []

key-decisions:
  - "Annotated tags over lightweight — store tagger info, work better with git describe"
  - "git push --follow-tags over separate git push origin vX.Y.Z — single command pushes commit and tag together"
  - "Keep a Changelog format — industry standard, produces GitHub Release description directly from CHANGELOG entries"
  - "Semver pre-1.0 rules documented: MINOR for features, PATCH for fixes — no stability guarantees yet"

patterns-established:
  - "Release commit format: release: vX.Y.Z (conventional commit)"
  - "All 3 version files (package.json, Cargo.toml, tauri.conf.json) bumped in same commit"
  - "Changelog [Unreleased] section accumulates next-release entries continuously"

requirements-completed: [REL-03, REL-04]

# Metrics
duration: 4min
completed: 2026-03-02
---

# Phase 14 Plan 01: Release Workflow Summary

**RELEASING.md runbook and CHANGELOG.md template establishing a complete, repeatable release process for VoiceType — version bump 3 files, commit, annotated tag, push triggers CI**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-03-02T20:18:45Z
- **Completed:** 2026-03-02T20:21:32Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- RELEASING.md provides an unambiguous 6-step runbook: prerequisites check, version bump (all 3 files), changelog update, commit, annotated tag, push
- CHANGELOG.md follows Keep a Changelog 1.1.0 format with [Unreleased] accumulation section and v0.1.0 baseline entry documenting MVP features
- Troubleshooting section covers the 3 most likely failure modes: CI not triggering, version mismatch, signing failure
- Runbook links directly to the CI pipeline behavior (what happens after push) so the release loop is fully documented end-to-end

## Task Commits

Each task was committed atomically:

1. **Task 1: Create RELEASING.md release runbook** - `f3278b3` (feat)
2. **Task 2: Create CHANGELOG.md with template format** - `5926675` (feat)

**Plan metadata:** (final commit below)

## Files Created/Modified

- `RELEASING.md` - Step-by-step release runbook: prerequisites, version bump (3 files), changelog update, commit/tag/push commands, CI description, semver rules, troubleshooting
- `CHANGELOG.md` - Keep a Changelog format with [Unreleased] section, v0.1.0 baseline entry (12 features documented), and footer comparison links for GitHub

## Decisions Made

- **Annotated tags** over lightweight: annotated tags store tagger identity and timestamp, work with `git describe`, and are better practice for release tagging
- **`git push --follow-tags`** over separate `git push origin vX.Y.Z`: single command that pushes the commit and all annotated tags pointing to reachable commits — no chance of forgetting the tag push
- **Keep a Changelog** format: industry standard that maps directly to GitHub Release description — changelog entry can be copied as-is into release notes
- **Changelog update step** placed before commit: ensures the release commit bundles both the version bump and the changelog entry atomically

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Release process is fully documented and ready for the first production release
- v0.1.0 tag can be pushed at any time following RELEASING.md to produce the first CI-built installer
- Phase 14 milestone complete — v1.1 Auto-Updates & CI/CD milestone fully delivered
- Future work: Claude skill for automated release flow (identified as deferred in context)

---
*Phase: 14-release-workflow*
*Completed: 2026-03-02*
