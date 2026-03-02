# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v1.0 — MVP

**Shipped:** 2026-03-02
**Phases:** 10 | **Plans:** 26 | **Quick Tasks:** 16
**Timeline:** 4 days (2026-02-27 → 2026-03-02) | **Commits:** 237

### What Was Built
- Full local voice-to-text desktop app with Tauri 2.0 (Rust + React)
- Dual transcription engines: Whisper (CUDA) and Parakeet TDT (CUDA/DirectML)
- Glassmorphism pill overlay with sinusoidal frequency bars, animated state transitions
- Hold-to-talk and toggle mode with Silero VAD silence detection
- Vocabulary profiles with structural engineering domain support + word correction dictionary
- First-run setup with GPU auto-detection, model download with progress, NSIS installer
- System tray with custom microphone icon and state-colored transitions

### What Worked
- Bottom-up dependency ordering (foundation → audio → pipeline → UI → settings → distribution) prevented integration issues
- Quick tasks for post-phase polish (pill position, badge changes, GPU acceleration) kept phases focused on core deliverables
- Decimal phase numbering (4.1, 6.1) cleanly handled urgent insertions without disrupting numbering
- Feature-gating Parakeet behind cargo features allowed incremental engine addition
- Human verification plans (06-04, 06.1-02, 07-03) caught runtime bugs that unit tests wouldn't

### What Was Inefficient
- Phase 06.2 (neon waveform) was built and reverted — wasted a full plan cycle
- Parakeet int8 model was added (quick-13) then removed entirely (quick-19) — should have started with fp32 only
- Fastest badge was added (08-03) then removed (quick-17) — badge strategy changed mid-milestone
- ROADMAP.md progress table drifted from actual disk state on decimal phases (4.1, 6.1, 8) — manual tracking couldn't keep up
- Some decisions in STATE.md accumulated faster than they could be cleaned — 120+ decision entries by end

### Patterns Established
- `Arc<Mutex<Option<T>>>` pattern for hot-swappable managed state (WhisperStateMutex, ParakeetStateMutex, AudioCaptureMutex)
- Channel<Event> pattern for streaming progress from Rust to React (download progress, audio levels)
- `CachedGpuMode` / `CachedGpuDetection` for startup-cached hardware detection
- Provider string API for ONNX execution providers ("cuda"/"directml"/"cpu")
- Atomic write pattern (.tmp-then-rename) for model downloads
- Fresh VAD instance per call to prevent LSTM state contamination

### Key Lessons
1. Start with the simplest model variant (fp32) before adding optimized variants (int8) — avoids build-then-remove churn
2. Human verification plans are essential for UI-heavy phases — automated tests can't catch visual/UX regressions
3. Phase insertions (decimal phases) work well for urgent fixes but ROADMAP tracking needs automation
4. Quick tasks are the right vehicle for post-phase polish — keeps phase plans focused on core scope
5. ort version pinning conflicts (parakeet-rs vs voice_activity_detector) are a real constraint — check dependency graphs before adding ONNX crates

### Cost Observations
- Model mix: predominantly sonnet for execution, opus for planning/complex decisions
- 237 commits across ~20 sessions over 4 days
- Notable: quick tasks (16 total) accounted for significant post-phase refinement

---

## Milestone: v1.1 — Auto-Updates & CI/CD

**Shipped:** 2026-03-02
**Phases:** 4 | **Plans:** 5
**Timeline:** 1 day (2026-03-02) | **Commits:** 31

### What Was Built
- Ed25519 signing infrastructure (keypair, GitHub secrets, public key in tauri.conf.json)
- Public GitHub repo kkosiak592/voicetype
- tauri-plugin-updater + tauri-plugin-process with Rust backend and JS frontend
- UpdateBanner component with full update lifecycle (check, download with progress, install, relaunch)
- GitHub Actions CI/CD workflow with CUDA 12.6.3 + LLVM 18 build environment
- RELEASING.md runbook and CHANGELOG.md template

### What Worked
- Linear phase dependency chain (signing → plugin → CI → docs) meant no integration surprises
- Splitting plugin integration into two plans (Rust backend, then frontend UI) kept each plan tightly scoped
- Human checkpoint in Phase 11 (private key backup confirmation) was the right gate for irreversible security step
- JS plugin API for download (instead of Rust IPC) simplified progress callback wiring
- tauri-action handled signing + latest.json generation — no custom build scripts needed
- CUDA minimal sub-packages in CI avoided 4 GB download while providing all needed headers

### What Was Inefficient
- Updater config initially placed under wrong tauri.conf.json key (v1 format instead of v2 plugins section) — caught post-checkpoint but should have been caught during planning
- Phase numbering jumped from 8 to 11 (gap of 9-10 not used) — legacy from roadmap creation
- Mid-download close limitation (banner resets to available state) acknowledged but not resolved — plugin API limitation

### Patterns Established
- Plugin registration split: setup()-requiring plugins in setup(), stateless plugins on Builder chain
- Tray menu rebuild pattern: Tauri 2 menus are immutable, so create new Menu + set_menu() for updates
- Keep a Changelog format for consistent release notes
- Annotated git tags with git push --follow-tags for atomic release publishing

### Key Lessons
1. Verify Tauri config format version (v1 vs v2) at planning time — config placement errors are silent until runtime
2. Minimal CI dependency installation (sub-packages over full toolkit) significantly reduces build times
3. Infrastructure milestones (signing, CI, docs) are low-risk and fast when the app code is stable
4. tauri-action abstracts most release complexity — custom build scripts are rarely needed for standard Tauri apps

### Cost Observations
- Model mix: predominantly sonnet for execution, opus for phase planning
- Completed in a single session (~3 hours wall clock)
- Notable: all 5 plans executed with zero deviations from plan — infrastructure work is highly predictable

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Sessions | Phases | Key Change |
|-----------|----------|--------|------------|
| v1.0 | ~20 | 10 | Initial process — bottom-up build with quick task polish |
| v1.1 | 1 | 4 | Infrastructure milestone — linear dependencies, zero deviations |

### Top Lessons (Verified Across Milestones)

1. Human verification checkpoints catch issues automated tests miss (v1.0: UI bugs; v1.1: config format)
2. Phase dependency ordering (bottom-up in v1.0, linear in v1.1) prevents integration surprises when respected
3. Planning pays off — phases that follow plans closely execute fastest (v1.1: 0 deviations across 5 plans)
