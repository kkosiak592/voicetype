# Phase 14: Release Workflow - Context

**Gathered:** 2026-03-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Document the exact steps to cut a release (version bump → commit → tag → push) and establish a consistent changelog/release notes format. The CI/CD pipeline (Phase 13) handles everything after the push — this phase handles everything before it and documents the full process.

</domain>

<decisions>
## Implementation Decisions

### Version bump process
- Manual edits to 3 files: `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`
- Runbook lists the exact files and fields to update
- Flow: edit versions → commit → tag → push (CI pipeline triggers on `v*` tag)

### Changelog format
- Claude's discretion on entry creation method, location, categories, and metadata
- Must produce consistent GitHub Release descriptions (REL-04)

### Runbook
- Lives at `RELEASING.md` in repo root
- Must list exact commands to cut a release with no gaps or ambiguity (REL-03)

### Claude's Discretion
- Versioning scheme (semver rules for a desktop app)
- Git tag type (annotated vs lightweight)
- Changelog entry creation method (manual vs commit-derived)
- Changelog location (repo file, GitHub Releases, or both)
- Release notes categories and metadata
- Runbook detail level and whether to include pre-checks
- Rollback/recovery instructions inclusion
- GitHub Release title format
- Full changelog diff link inclusion
- Whether CI auto-creates GitHub Release or runbook includes manual `gh release create`

</decisions>

<specifics>
## Specific Ideas

- User wants a Claude skill (project-level) to eventually automate the full release flow — version bump, commit, tag, build, bundle. The runbook establishes what that skill will need to automate.
- No specific reference apps for release notes style — standard conventions are fine.
- User wants simple: "commit → tag → push" with CI handling the rest.

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `tauri.conf.json`: Version field at top level, updater config with GitHub Releases endpoint already configured
- Phase 13 CI/CD pipeline: Will create GitHub Actions workflow triggered by `v*` tags, producing signed installer + `latest.json`

### Established Patterns
- Version tracked in 3 places: `package.json` (0.1.0), `src-tauri/Cargo.toml` (0.1.0), `src-tauri/tauri.conf.json` (0.1.0)
- Ed25519 signing already configured (Phase 11)
- Updater endpoint: `https://github.com/kkosiak592/voicetype/releases/latest/download/latest.json`
- Tag pattern: `v*` (e.g., `v0.1.0`, `v1.0.0`)
- GitHub remote: `kkosiak592/voicetype`

### Integration Points
- CI/CD workflow (Phase 13) triggers on tag push — runbook must produce tags that match the expected `v*` pattern
- `tauri.conf.json` version must match the tag version for updater to work correctly
- No existing CHANGELOG.md — will be created fresh

</code_context>

<deferred>
## Deferred Ideas

- Claude skill for automated release flow (version bump → commit → tag → build → bundle) — to be created after runbook establishes the process
- This is a project-level skill using the skill-creator workflow

</deferred>

---

*Phase: 14-release-workflow*
*Context gathered: 2026-03-02*
