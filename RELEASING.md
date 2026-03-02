# Releasing VoiceType

Version bump, commit, tag, push — CI handles the rest.

---

## Prerequisites

Before cutting a release, ensure:

- `git status` shows a clean working tree (no uncommitted changes)
- You are on the `master` branch (`git branch --show-current`)
- All changes for this release are merged into `master`

---

## Steps

### 1. Decide the version

Follow [Semantic Versioning](https://semver.org/):

| Bump | When |
|------|------|
| `MAJOR` | Breaking changes to user-facing behavior or data formats |
| `MINOR` | New features, new settings, new capabilities |
| `PATCH` | Bug fixes, performance improvements, dependency updates |

**Pre-1.0 note (current):** Use `MINOR` for features, `PATCH` for fixes. No stability guarantees yet.

---

### 2. Bump the version in all 3 files

All three files MUST have the same version, and it MUST match the tag you push (without the `v` prefix).

**`package.json`** — line 4:
```json
"version": "X.Y.Z",
```

**`src-tauri/Cargo.toml`** — line 3:
```toml
version = "X.Y.Z"
```

**`src-tauri/tauri.conf.json`** — line 4:
```json
"version": "X.Y.Z",
```

---

### 3. Update CHANGELOG.md

1. Create a new section below `[Unreleased]` with the version and today's date:
   ```markdown
   ## [X.Y.Z] - YYYY-MM-DD
   ```
2. Move all entries from `[Unreleased]` into the new section under the appropriate categories (`Added`, `Changed`, `Fixed`, `Removed`).
3. Leave `[Unreleased]` empty above the new section — it accumulates the next release.
4. Update the footer comparison links:
   ```markdown
   [Unreleased]: https://github.com/kkosiak592/voicetype/compare/vX.Y.Z...HEAD
   [X.Y.Z]: https://github.com/kkosiak592/voicetype/releases/tag/vX.Y.Z
   ```

The changelog entry for the new version will be used as the GitHub Release description.

---

### 4. Commit

```bash
git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json CHANGELOG.md
git commit -m "release: vX.Y.Z"
```

---

### 5. Tag

Use an annotated tag (stores tagger info, works better with `git describe`):

```bash
git tag -a vX.Y.Z -m "vX.Y.Z"
```

---

### 6. Push

Push the commit and tag together — this triggers the GitHub Actions release workflow:

```bash
git push origin master --follow-tags
```

---

## What Happens Next (CI Pipeline)

After the tag push, GitHub Actions automatically:

1. Checks out the repo on a Windows runner
2. Installs CUDA toolkit and LLVM/clang for the build environment
3. Builds the NSIS installer (`.exe`) with Rust + Tauri
4. Signs the installer with the Ed25519 key from repo secrets — produces a `.exe.sig` signature file
5. Generates `latest.json` with download URLs and signature for the auto-updater
6. Publishes a GitHub Release titled `VoiceType vX.Y.Z` with the installer, signature, and `latest.json` attached
7. Existing installs will detect the update on next launch via the updater endpoint:
   `https://github.com/kkosiak592/voicetype/releases/latest/download/latest.json`

Build time: approximately 20–40 minutes.

---

## Troubleshooting

**CI didn't trigger after push:**
- Verify the tag matches the `v*` pattern (e.g., `v1.0.0` not `1.0.0`)
- Check the Actions tab in the GitHub repo for any errors
- Confirm the tag was pushed: `git ls-remote --tags origin`

**Version mismatch error:**
- All three files (`package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`) and the tag must have the same version string
- Tag: `vX.Y.Z` → files: `X.Y.Z` (no `v` prefix in files)

**Signing failed in CI:**
- Check that `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` secrets are set in the repo's Settings → Secrets and variables → Actions
- Verify the private key value is the full base64-encoded key (not a file path)

**Auto-updater not showing update:**
- Confirm `latest.json` was uploaded to the GitHub Release
- Check that the version in `latest.json` is higher than the installed version
- The updater checks on app launch; restart the app to trigger a check
