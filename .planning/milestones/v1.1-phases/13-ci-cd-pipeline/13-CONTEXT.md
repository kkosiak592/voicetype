# Phase 13: CI/CD Pipeline - Context

**Gathered:** 2026-03-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Pushing a version tag (`v*`) triggers a fully automated GitHub Actions pipeline that produces a signed Windows NSIS installer, a valid `latest.json` with Ed25519 signature, and a published GitHub Release with all assets attached. No manual steps after the tag push.

</domain>

<decisions>
## Implementation Decisions

### Release binary features
- Ship both transcription engines: whisper-rs (CUDA) and parakeet-rs (CUDA + DirectML)
- Build with all default feature flags: `whisper` + `parakeet`
- Both GPU backends enabled: CUDA for NVIDIA, DirectML for AMD/Intel
- Models are NOT bundled in the installer — downloaded on first run (existing app infrastructure handles this)

### Release gating
- Publish immediately on workflow completion — no draft review step
- No test/validation step in the release pipeline — build and release only
- Tag push is the sole trigger and gate

### Build environment
- GitHub-hosted Windows runner (`windows-latest`)
- Public repo — unlimited free Actions minutes
- 20-40 min build time acceptable for releases
- CUDA toolkit + LLVM/clang must be installed in workflow (CI runner doesn't have these)

### Claude's Discretion
- CUDA toolkit installation method in CI (setup action vs manual install)
- Rust/cargo caching strategy (basic or aggressive)
- Release notes format and categorization approach
- LLVM/clang installation method
- Workflow file structure and job organization

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. User wants a simple, working pipeline over an optimized one.

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `tauri.conf.json`: Already configured with NSIS target, updater pubkey, and GitHub releases endpoint
- `createUpdaterArtifacts: "v1Compatible"`: latest.json generation is built into Tauri's bundle process
- `package.json` build script: `tsc && vite build` for frontend
- `Cargo.toml`: Feature flags `whisper` (CUDA) and `parakeet` (CUDA + DirectML) already defined

### Established Patterns
- Ed25519 signing key already in GitHub secrets (CICD-06 complete: `TAURI_SIGNING_PRIVATE_KEY` + password)
- Updater endpoint: `https://github.com/kkosiak592/voicetype/releases/latest/download/latest.json`
- Local patches committed to repo (`src-tauri/patches/esaxx-rs`, `src-tauri/patches/parakeet-rs`) — CI picks these up via `[patch.crates-io]` in Cargo.toml

### Integration Points
- GitHub remote: `kkosiak592/voicetype` (origin)
- Tag pattern: `v*` (e.g., `v0.1.0`, `v1.0.0`)
- Version in three places: `package.json`, `Cargo.toml`, `tauri.conf.json` (all currently `0.1.0`)
- Build dependencies: CUDA Toolkit, LLVM/clang (for bindgen), cmake, Node.js, Rust toolchain

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 13-ci-cd-pipeline*
*Context gathered: 2026-03-02*
