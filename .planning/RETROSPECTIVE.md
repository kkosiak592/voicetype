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

## Milestone: v1.2 — Keyboard Hook

**Shipped:** 2026-03-07
**Phases:** 10 (1 voided) | **Plans:** 15 | **Quick Tasks:** 21
**Timeline:** 5 days (2026-03-02 -> 2026-03-07) | **Commits:** 253

### What Was Built
- WH_KEYBOARD_LL low-level keyboard hook on dedicated thread with 50ms debounce and Start menu suppression
- Runtime routing between hook (modifier-only combos) and tauri-plugin-global-shortcut (standard combos)
- Frontend modifier-only combo capture with progressive display and canonical token ordering
- Moonshine Tiny ONNX as third transcription engine with download, selection, and GPU support
- Engine-agnostic VAD chunking for 60s+ recordings across Whisper, Parakeet, and Moonshine
- Data-driven model selection with benchmark stats and universal parakeet recommendation
- UI polish: tray icon fixes, profile simplification, transcription history panel, pill drag reposition
- CUDA DLLs bundled in single installer with runtime GPU fallback
- Filler word removal, always-listen mode, corrections dictionary inline editor

### What Worked
- Win32 keyboard hook research (five critical pitfalls) front-loaded into Phase 15 planning prevented all common hook failure modes
- Benchmark-driven model decisions (Quick tasks 27-37) provided empirical data for model selection revamp
- Decimal phase numbering (19.1, 19.2, 19.3, 20.1) scaled well for scope additions without disrupting execution order
- Quick tasks (21 total) handled feature polish, benchmarking, and small fixes efficiently alongside phased work
- Conditional routing pattern (is_modifier_only) cleanly separated hook vs global-shortcut code paths

### What Was Inefficient
- Phase 18 (Integration and Distribution) was planned, voided, and moved to Phase 21 — which was also never executed. DIST-01 (VirusTotal scan) deferred as known gap
- distil-large-v3.5 was added in Phase 19 then removed in Phase 19.2 — similar to the v1.0 int8 add-then-remove pattern
- Phase 20 scope originally included dual CPU/GPU installers — simplified to single installer mid-planning
- ROADMAP progress table columns drifted again on decimal phases — same issue as v1.0

### Patterns Established
- `handle_hotkey_event(app, pressed: bool)` as shared entry point for both hook and global-shortcut paths
- `HookHandleState(Mutex<Option<HookHandle>>)` for cross-boundary hook lifecycle management
- `vad_chunk_audio(samples, max_seconds)` as engine-agnostic chunking function
- `TAURI_CONFIG` env var injection in CI for build-time-only bundle configuration
- Correction log module for promoting inline corrections to persistent dictionary

### Key Lessons
1. Avoid add-then-remove churn by validating model/feature viability with benchmarks BEFORE integration (distil-large-v3.5 repeated v1.0's int8 mistake)
2. Win32 API research is high-value — understanding pitfalls before coding prevented all 5 known hook failure modes
3. Single-installer approach (bundle DLLs, runtime fallback) is simpler and more maintainable than installer variants
4. Engine-agnostic abstractions (VAD chunking) should be built from the start, not generalized after the fact
5. Phase voiding/movement (18 -> 21) signals scope uncertainty — better to defer unresolved phases entirely during planning

### Cost Observations
- Model mix: opus for planning/complex decisions, sonnet for execution, balanced profile
- 253 commits across ~10 sessions over 5 days
- Notable: 21 quick tasks — highest count per milestone, reflecting feature breadth beyond original keyboard hook scope

---

## Milestone: v1.3 — Clipboard Simplification

**Shipped:** 2026-03-07
**Phases:** 1 | **Plans:** 1
**Timeline:** 1 day (2026-03-07) | **Commits:** 2

### What Was Built
- Removed clipboard save/restore from inject_text — transcription stays on clipboard
- Eliminated 80ms post-paste sleep that existed only for restore timing
- Simplified injection to 3-step flow: set clipboard, verify, paste

### What Worked
- Smallest possible milestone scope — one phase, one plan, net -24 lines
- Matched standard dictation tool behavior (transcription on clipboard is expected)

### What Was Inefficient
- Nothing — clean, minimal scope

### Key Lessons
1. Subtractive milestones (removing code) can be as valuable as additive ones

### Cost Observations
- Single session, ~5 minutes execution time
- Notable: simplest milestone to date

---

## Milestone: v1.4 — Per-App Settings

**Shipped:** 2026-03-07
**Phases:** 4 | **Plans:** 5
**Timeline:** 5 days (2026-03-03 -> 2026-03-07) | **Execution time:** 33 min

### What Was Built
- Win32 foreground detection (GetForegroundWindow chain) with UWP resolution via EnumChildWindows
- Pure resolve_all_caps() override function with 8 unit tests and safe lock ordering
- App Rules settings page with color-coded three-state dropdown and detect-app countdown flow
- Browse Running Apps searchable dropdown with CreateToolhelp32Snapshot process enumeration
- Per-app rules persistence via settings.json with startup hydration

### What Worked
- Pure function for override resolution enabled 8 unit tests without Win32 dependencies
- Lock ordering discipline (ActiveProfile before AppRulesState) prevented deadlocks by design
- Case-normalizing exe names at every boundary eliminated matching bugs
- Four tightly-scoped phases with clear dependency chain (detect → pipeline → UI → dropdown)
- All 5 plans executed with minimal deviations (3 auto-fixed bugs, 0 scope creep)

### What Was Inefficient
- Nothing significant — clean execution with 33 minutes total across 5 plans

### Patterns Established
- `resolve_all_caps(profile_val, exe_name, rules)` — pure function override resolution pattern
- `Option<bool>` three-state toggle: None=inherit, Some(true)=ON, Some(false)=OFF
- Two-phase process enumeration: EnumWindows for visible windows, CreateToolhelp32Snapshot for exe names
- Inline button state machine: idle/countdown/success/error with auto-reset

### Key Lessons
1. Pure function resolution + unit tests is the right pattern for override logic — avoids Win32 test dependencies
2. Case normalization at every boundary (set, remove, lookup) is essential for exe name matching
3. Fetch-once strategy for process dropdown is sufficient — no need for auto-refresh complexity
4. Custom dropdowns (vs native select) are worth the effort for color-coded multi-state controls

### Cost Observations
- Model mix: balanced profile
- 33 minutes total execution across 5 plans — fastest execution per plan
- Notable: no quick tasks needed — phases covered all requirements directly

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Sessions | Phases | Quick Tasks | Commits | Key Change |
|-----------|----------|--------|-------------|---------|------------|
| v1.0 | ~20 | 10 | 16 | 237 | Initial process — bottom-up build with quick task polish |
| v1.1 | 1 | 4 | 0 | 31 | Infrastructure milestone — linear dependencies, zero deviations |
| v1.2 | ~10 | 10 | 21 | 253 | Scope growth beyond original goal — benchmark-driven model decisions |
| v1.3 | 1 | 1 | 0 | 2 | Subtractive milestone — net -24 lines, simplest execution |
| v1.4 | ~5 | 4 | 0 | ~10 | Cleanest execution — 33 min total, 0 scope creep, pure function testing |

### Top Lessons (Verified Across Milestones)

1. Human verification checkpoints catch issues automated tests miss (v1.0: UI bugs; v1.1: config format; v1.2: hook pitfalls)
2. Phase dependency ordering prevents integration surprises when respected (all milestones)
3. Planning pays off — phases that follow plans closely execute fastest (v1.1: 0 deviations; v1.2: hook phases executed cleanly)
4. Avoid add-then-remove churn — validate viability before integration (v1.0: int8 model; v1.2: distil-large-v3.5)
5. Quick tasks scale well for feature polish but can expand milestone scope significantly (v1.2: 21 quick tasks added filler removal, always-listen, pill drag, corrections editor)
