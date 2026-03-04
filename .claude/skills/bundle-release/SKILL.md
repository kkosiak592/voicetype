---
name: bundle-release
description: >
  Full release packaging workflow for VoiceType. Checks working tree, suggests
  semver bump, updates version files and CHANGELOG.md, commits, tags, and pushes
  to trigger the GitHub Actions release pipeline. Every step requires explicit
  user approval before proceeding. Use when the user says "release", "bundle",
  "cut a release", "push a version", "ship it", or invokes /bundle-release.
---

# Bundle Release

Automated release workflow. Each step gates on user approval before proceeding.

## Step 1 — Working tree check

```bash
git status --short
git branch --show-current
```

Report:
- Current branch (must be `master` — warn if not)
- Untracked files, staged changes, unstaged changes
- For each category, recommend: commit, stash, or ignore

**Gate:** Ask user to resolve any dirty state or confirm proceeding as-is.

## Step 2 — Determine version bump

Read current version from `package.json` (line 4, `"version"` field).

Gather context for suggestion:
```bash
git log --oneline $(git describe --tags --abbrev=0 2>/dev/null || git rev-list --max-parents=0 HEAD)..HEAD
```

Analyze commits since last tag. Suggest a bump using semver rules:
- **MAJOR** — breaking changes to user-facing behavior or data formats
- **MINOR** — new features, settings, capabilities
- **PATCH** — bug fixes, perf improvements, dependency updates
- Pre-1.0: use MINOR for features, PATCH for fixes

Present: current version, suggested next version, reasoning (list key commits that drove the suggestion).

**Gate:** Ask user to approve the suggested version or provide a different one.

## Step 3 — Update version in all 3 files

Replace the version string in:
1. `package.json` — `"version": "X.Y.Z"`
2. `src-tauri/Cargo.toml` — `version = "X.Y.Z"`
3. `src-tauri/tauri.conf.json` — `"version": "X.Y.Z"`

After editing, verify all three match:
```bash
grep '"version"' package.json | head -1
grep '^version' src-tauri/Cargo.toml | head -1
grep '"version"' src-tauri/tauri.conf.json | head -1
```

Report the three values. They must be identical.

**Gate:** Ask user to confirm the version updates look correct.

## Step 4 — Update CHANGELOG.md

Read `CHANGELOG.md`. Generate a new release section below `[Unreleased]`:

```markdown
## [X.Y.Z] - YYYY-MM-DD
```

Populate it by categorizing commits since the last tag into `Added`, `Changed`, `Fixed`, `Removed` (omit empty categories). Write human-readable descriptions, not raw commit messages.

Move all content from `[Unreleased]` into the new section. Leave `[Unreleased]` empty.

Update footer comparison links:
```markdown
[Unreleased]: https://github.com/kkosiak592/voicetype/compare/vX.Y.Z...HEAD
[X.Y.Z]: https://github.com/kkosiak592/voicetype/compare/vPREV...vX.Y.Z
```

Present the full changelog diff to the user.

**Gate:** Ask user to approve the changelog or request edits.

## Step 5 — Commit

Stage and commit:
```bash
git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json CHANGELOG.md
git commit -m "release: vX.Y.Z"
```

Show the commit hash and summary.

**Gate:** Ask user to confirm the commit looks correct before tagging.

## Step 6 — Tag

Create an annotated tag:
```bash
git tag -a vX.Y.Z -m "vX.Y.Z"
```

Confirm the tag was created:
```bash
git tag -l --sort=-v:refname | head -3
```

**Gate:** Ask user to approve pushing. Remind them this triggers the CI release pipeline and is the point of no return.

## Step 7 — Push

```bash
git push origin master --follow-tags
```

Report success and provide:
- Link: `https://github.com/kkosiak592/voicetype/actions` (monitor build)
- Link: `https://github.com/kkosiak592/voicetype/releases` (check release when done)
- Expected build time: ~20-40 minutes

## Abort

If the user says "stop", "abort", or "cancel" at any gate:
- Do NOT proceed to the next step
- If a commit was already made but not pushed, offer to undo: `git reset --soft HEAD~1 && git tag -d vX.Y.Z`
- Report what was completed and what was rolled back
