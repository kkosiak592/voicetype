# Architecture Review — VoiceType

**Date:** 2026-03-01
**Scope:** Full codebase (`src-tauri/src/`, `src/`)
**Reviewer:** Architecture Review Agent

---

## 1. Architecture Overview

VoiceType is a Tauri v2 desktop app (Rust backend + React/TypeScript frontend) providing offline voice-to-text dictation on Windows. The app has two windows: a settings panel and a floating pill overlay.

### Backend Modules (Rust, `src-tauri/src/`)

| Module | Responsibility | Lines |
|---|---|---|
| `lib.rs` | App bootstrap, Tauri commands, global shortcut handlers, managed state registration, settings I/O | ~1318 |
| `audio.rs` | Persistent microphone capture, resampling (cpal + rubato), buffer management | ~269 |
| `pipeline.rs` | Pipeline state machine (IDLE/RECORDING/PROCESSING), orchestration of VAD -> transcribe -> inject | ~254 |
| `vad.rs` | Silero V5 voice activity detection (post-hoc gate + streaming worker for toggle mode) | ~291 |
| `transcribe.rs` | Whisper model loading, GPU detection (NVML), inference (feature-gated behind `whisper`) | ~181 |
| `inject.rs` | Text injection via clipboard save/paste/restore (arboard + enigo) | ~59 |
| `corrections.rs` | Regex-based word-boundary corrections engine | ~76 |
| `profiles.rs` | Vocabulary profile definitions (structural engineering, general) | ~84 |
| `tray.rs` | System tray icon state management and menu | ~70 |
| `pill.rs` | Pill window positioning, RMS level streaming | ~103 |
| `download.rs` | Model file download with streaming progress and SHA256 validation | ~200 |
| `corrections_tests.rs` | Unit tests for corrections engine and profiles | ~93 |

### Frontend Modules (TypeScript/React, `src/`)

| Module | Responsibility |
|---|---|
| `main.tsx` / `pill-main.tsx` | Entry points for settings and pill windows |
| `App.tsx` | Settings root — first-run gate, section routing, state management |
| `Pill.tsx` | Pill overlay — event-driven state machine (hidden/recording/processing/error) |
| `lib/store.ts` | Tauri plugin-store wrapper for settings persistence |
| `components/FirstRun.tsx` | First-run model download flow |
| `components/ModelSelector.tsx` | Model selection + download UI for settings |
| `components/Sidebar.tsx` | Settings navigation sidebar |
| `components/sections/*.tsx` | Individual settings sections (General, Profiles, Model, Microphone, Appearance) |
| `components/HotkeyCapture.tsx` | Hotkey rebinding UI |
| `components/RecordingModeToggle.tsx` | Hold/toggle mode selector |
| `components/ProfileSwitcher.tsx` | Profile selection cards |
| `components/DictionaryEditor.tsx` | Corrections dictionary table editor |
| `components/ThemeToggle.tsx` | Dark mode toggle |
| `components/AutostartToggle.tsx` | Autostart toggle |
| `components/FrequencyBars.tsx` | Animated audio level visualization |
| `components/ProcessingDots.tsx` | Processing state animation |

---

## 2. Data Flow Diagram

```
                    ┌──────────────────────────────┐
                    │   Global Shortcut (OS-level)  │
                    └─────────────┬────────────────┘
                                  │ Pressed / Released
                                  ▼
                    ┌──────────────────────────────┐
                    │         lib.rs                │
                    │   Hotkey Handler (setup +     │
                    │   rebind_hotkey)              │
                    │                              │
                    │   PipelineState CAS:         │
                    │   IDLE -> RECORDING ->       │
                    │   PROCESSING                 │
                    └──┬──────┬────────┬───────────┘
                       │      │        │
            ┌──────────┘      │        └──────────────┐
            ▼                 ▼                        ▼
  ┌─────────────────┐  ┌──────────────┐  ┌─────────────────────┐
  │   audio.rs      │  │   pill.rs    │  │   tray.rs           │
  │   AudioCapture  │  │  show_pill() │  │  set_tray_state()   │
  │  .recording=T   │  │  level_stream│  │  icon + tooltip     │
  │  .buffer fills  │  │  -> pill-level│  └─────────────────────┘
  └────────┬────────┘  └──────┬───────┘
           │                  │
           │  (on release /   │  emit_to("pill", ...)
           │   second tap)    │
           ▼                  ▼
  ┌──────────────────────────────────────┐
  │           pipeline.rs                │
  │  run_pipeline(app):                  │
  │    1. flush_and_stop() -> samples    │
  │    2. vad::vad_gate_check(samples)   │
  │    3. transcribe_audio(ctx, samples) │
  │    4. corrections.apply(text)        │
  │    5. inject::inject_text(text)      │
  │    6. reset_to_idle()                │
  └──┬────────┬────────┬────────┬───────┘
     │        │        │        │
     ▼        ▼        ▼        ▼
  vad.rs  transcribe corrections inject.rs
          .rs        .rs        (clipboard
                                + Ctrl+V)
```

```
  Frontend (Settings Window)
  ┌──────────────────────────────────────────┐
  │ App.tsx                                  │
  │  ├── FirstRun.tsx ─── invoke("download_  │
  │  │                     model", Channel)  │
  │  │                                       │
  │  ├── GeneralSection                      │
  │  │    ├── HotkeyCapture ── invoke(       │
  │  │    │                    "rebind_hotkey")
  │  │    └── RecordingModeToggle ── invoke( │
  │  │                       "set_recording_ │
  │  │                        mode")         │
  │  ├── ProfilesSection                     │
  │  │    ├── ProfileSwitcher ── invoke(     │
  │  │    │               "set_active_profile")
  │  │    └── DictionaryEditor ── invoke(    │
  │  │                "save_corrections")    │
  │  ├── ModelSection                        │
  │  │    └── ModelSelector ── invoke(       │
  │  │                    "set_model",       │
  │  │                    "download_model")  │
  │  ├── MicrophoneSection ── invoke(        │
  │  │              "set_microphone")        │
  │  └── AppearanceSection                   │
  │       ├── ThemeToggle (local store)      │
  │       └── AutostartToggle (plugin)       │
  └──────────────────────────────────────────┘

  Frontend (Pill Window)
  ┌──────────────────────────────────────────┐
  │ Pill.tsx                                 │
  │  listens: pill-show, pill-hide,          │
  │           pill-state, pill-level,        │
  │           pill-result                    │
  │  ├── FrequencyBars (recording)           │
  │  └── ProcessingDots (processing)         │
  └──────────────────────────────────────────┘
```

---

## 3. Module Dependency Map

### Rust Module Dependencies

```
lib.rs ──────► audio.rs       (AudioCaptureMutex, start_persistent_stream)
         ├───► pipeline.rs    (PipelineState, IDLE/RECORDING/PROCESSING)
         ├───► vad.rs         (VadWorkerHandle, spawn_vad_worker)
         ├───► tray.rs        (build_tray, set_tray_state, TrayState)
         ├───► pill.rs        (show_pill, start_level_stream)
         ├───► profiles.rs    (ActiveProfile, get_all_profiles, profile constructors)
         ├───► corrections.rs (CorrectionsState, CorrectionsEngine)
         ├───► download.rs    (download_model command)
         └───► transcribe.rs  (detect_gpu, resolve_model_path, load_whisper_context) [feature-gated]

pipeline.rs ──► audio.rs      (AudioCaptureMutex — via managed state)
           ├──► vad.rs        (vad_gate_check, VadWorkerHandle cancel)
           ├──► transcribe.rs (transcribe_audio — feature-gated)
           ├──► corrections.rs(CorrectionsState — via managed state)
           ├──► profiles.rs   (ActiveProfile — via managed state)
           ├──► inject.rs     (inject_text)
           ├──► tray.rs       (set_tray_state)
           └──► pill.rs       (implicit — via emit_to)

vad.rs ────────► pipeline.rs  (PipelineState, RECORDING/PROCESSING/IDLE constants, run_pipeline)
           ├──► tray.rs       (set_tray_state)
           └──► lib.rs        (LevelStreamActive — via managed state)

audio.rs ──────  (standalone — no internal module deps)
transcribe.rs ── (standalone — no internal module deps)
inject.rs ─────  (standalone — no internal module deps)
corrections.rs─  (standalone — no internal module deps)
profiles.rs ───  (standalone — no internal module deps)
tray.rs ───────  (standalone — no internal module deps)
pill.rs ───────  (standalone — no internal module deps)
download.rs ───  (standalone — no internal module deps)
```

### Near-Circular Dependency: `pipeline.rs` <-> `vad.rs`

`pipeline.rs` calls `vad::vad_gate_check()` and `vad::VadWorkerHandle::cancel()`.
`vad.rs` calls `crate::pipeline::PipelineState`, `crate::pipeline::run_pipeline()`, and pipeline constants.

This is **not a Rust compilation error** (both use fully-qualified `crate::` paths; Rust resolves within the same crate). However, it is a **logical circular dependency** — the two modules are tightly coupled and co-dependent. The codebase has a comment in `vad.rs:115-116` explicitly warning against adding a `use crate::pipeline;` import, acknowledging this coupling.

---

## 4. Findings

### ARCH-01: `lib.rs` is a God Module (~1318 lines)

**Severity:** High
**Location:** `src-tauri/src/lib.rs:1-1318`

`lib.rs` contains:
- 17 `#[tauri::command]` functions (settings CRUD, recording, transcription, model management)
- 6 `read_saved_*()` helper functions that manually parse `settings.json`
- The entire hotkey handler (~200 lines of business logic inline in closures)
- Complete duplicate hotkey handler in `rebind_hotkey()` (~120 lines, nearly identical to setup handler)
- App bootstrap and setup (~120 lines)
- Multiple managed state type definitions (`LevelStreamActive`, `RecordingMode`, `VadWorkerState`, `WhisperStateMutex`, `Mode` enum)

This module violates single responsibility. It's the routing layer, the settings persistence layer, the state definition layer, and the hotkey orchestration layer all in one file. The hotkey handler closure contains ~200 lines of pipeline orchestration logic that should be in a dedicated module.

**Impact:** Maintenance difficulty, high merge conflict risk, unclear ownership of concerns.

### ARCH-02: Hotkey Handler Duplication

**Severity:** High
**Location:** `lib.rs:1064-1193` (setup handler) vs `lib.rs:153-280` (rebind_hotkey handler)

The hotkey press/release logic is duplicated verbatim between the initial `setup()` registration and the `rebind_hotkey()` command. Both contain identical pipeline state transitions, audio buffer management, VAD worker spawning, pill/tray updates, and pipeline spawning. This is ~130 lines of duplicated business logic.

Any behavioral change to the hotkey handler must be applied in both locations or the app will have inconsistent behavior depending on whether the user has rebound their hotkey.

**Impact:** Bug introduction risk from inconsistent updates. Already a maintenance hazard.

### ARCH-03: Pipeline <-> VAD Bidirectional Coupling

**Severity:** Medium
**Location:** `pipeline.rs:4` (`use crate::vad`), `vad.rs:233-290` (`crate::pipeline::*`)

`pipeline.rs` calls `vad::vad_gate_check()` and cancels VAD workers.
`vad.rs` transitions pipeline state, reads pipeline constants, and spawns `pipeline::run_pipeline()`.

While not a compilation error (same crate), this bidirectional dependency means neither module can be understood, tested, or modified in isolation. The `trigger_auto_stop()` function in `vad.rs` essentially IS pipeline logic — it transitions state, updates tray/pill, and spawns the pipeline.

**Recommendation:** Extract a shared `PipelineController` or move `trigger_auto_stop()` into `pipeline.rs` as a public function that `vad.rs` calls, keeping the dependency unidirectional (vad -> pipeline, never pipeline -> vad for orchestration).

### ARCH-04: Settings Persistence Bypasses the Store Layer

**Severity:** Medium
**Location:** `lib.rs:75-86` (`read_saved_hotkey`), `lib.rs:90-110` (`read_saved_mode`), `lib.rs:331-399` (profile/corrections/caps readers), `lib.rs:121-132` (`set_recording_mode` writer)

The Rust backend reads and writes `settings.json` directly via `std::fs::read_to_string` + `serde_json`, while the frontend uses `@tauri-apps/plugin-store` (which manages the same file). This creates two independent access paths to the same file:

1. Backend: raw filesystem read/write with manual JSON merge
2. Frontend: `Store.load('settings.json')` with `autoSave: 100`

The frontend store uses different key names than the backend:
- Frontend: `recordingMode` (camelCase)
- Backend: `recording_mode` (snake_case) at `lib.rs:106,127`
- Frontend: `activeProfile`
- Backend: `active_profile_id` at `lib.rs:345,478`

This means the frontend and backend are reading/writing different keys in the same JSON file, creating phantom settings that serve no purpose for the other layer.

**Impact:** Settings written by the backend are invisible to the frontend store and vice versa. The file contains both `recordingMode` and `recording_mode` keys with potentially different values.

### ARCH-05: `download.rs` Duplicates `transcribe::models_dir()`

**Severity:** Low
**Location:** `download.rs:40-47` vs `transcribe.rs:16-18`

`download.rs` duplicates the `models_dir()` function to avoid a dependency on the feature-gated `transcribe` module. The comment at `download.rs:42-43` explicitly acknowledges this. While intentional, this means the model directory path is defined in two places. If the path convention changes, both must be updated.

**Recommendation:** Extract `models_dir()` into a tiny `paths.rs` module with no feature gates, imported by both `download.rs` and `transcribe.rs`.

### ARCH-06: Tauri Managed State as Service Locator (Implicit Dependencies)

**Severity:** Low
**Location:** Throughout `lib.rs`, `pipeline.rs`, `vad.rs`

All inter-module communication goes through `app.state::<T>()`, making dependencies implicit rather than explicit. For example, `pipeline.rs:run_pipeline()` takes only an `AppHandle` but internally resolves 5 different managed state types:
- `audio::AudioCaptureMutex` (line 50)
- `profiles::ActiveProfile` (line 94)
- `WhisperStateMutex` (line 104)
- `corrections::CorrectionsState` (line 167)
- `VadWorkerState` (line 227, via `cancel_stale_vad_worker`)

This is idiomatic Tauri, but it makes the actual dependency graph invisible at the type level. A function signature of `fn run_pipeline(app: AppHandle)` reveals nothing about what it actually needs.

**Impact:** Low — this is standard Tauri practice. But it means adding a new managed state or removing one produces no compile-time errors; failures happen at runtime.

### ARCH-07: Frontend State Management is Prop-Drilling Without Shared State

**Severity:** Low
**Location:** `App.tsx:19-27`

`App.tsx` holds all settings state (hotkey, theme, recordingMode, activeProfile, selectedMic, selectedModel) and prop-drills them through section components. Each section component calls `invoke()` to the backend AND writes to the frontend store independently.

For a settings-only UI of this size, this is adequate and not over-engineered. No finding here for action — just noting that if more cross-cutting state emerges, a context or state manager would be needed.

### ARCH-08: IPC Boundary is Clean (Positive Finding)

The Tauri IPC boundary is well-structured:
- All backend state mutations go through `#[tauri::command]` functions
- The pill window receives updates via event emission (`emit_to("pill", ...)`) — no direct state sharing
- The frontend never directly accesses backend internals
- Feature-gated commands (`#[cfg(feature = "whisper")]`) prevent unavailable commands from being registered

The event protocol for the pill window is well-defined: `pill-show`, `pill-hide`, `pill-state`, `pill-level`, `pill-result`. The pill is a pure event consumer with no backend calls.

### ARCH-09: Frontend Download Logic Duplication

**Severity:** Low
**Location:** `FirstRun.tsx:4-8` and `ModelSelector.tsx:12-16`

The `DownloadEvent` type and `formatMB()` utility are defined independently in both `FirstRun.tsx` and `ModelSelector.tsx`. Both implement download state machines with progress tracking against the same backend command.

**Recommendation:** Extract shared download types and utilities into a `lib/download.ts` module.

### ARCH-10: Missing Audio State Registration on Failure

**Severity:** Medium
**Location:** `lib.rs:1224-1233`

When audio capture initialization fails, the code logs a warning but does NOT register `AudioCaptureMutex` in managed state. The comment at line 1226-1233 acknowledges this: "commands will panic on missing state." This means any Tauri command that accesses `app.state::<AudioCaptureMutex>()` will panic if the microphone is unavailable at startup.

This is a crash path, not a graceful degradation. Commands like `start_recording`, `stop_recording`, `set_microphone`, and the hotkey handler all access this state.

---

## 5. Dependency Direction Summary

```
               lib.rs (orchestrator + commands)
              /   |    |    |     \      \      \
             v    v    v    v      v      v      v
         audio pipeline vad tray  pill profiles corrections
                 |  ↑   |
                 v  |   v
                 vad    pipeline  (bidirectional coupling)
                 |
                 v
              pipeline.run_pipeline
                /    |      |      \
               v     v      v       v
            audio  transcribe corrections inject
                   (feature-gated)
```

**Leaf modules (no internal deps):** `audio.rs`, `transcribe.rs`, `inject.rs`, `corrections.rs`, `profiles.rs`, `tray.rs`, `pill.rs`, `download.rs`

**Hub module:** `lib.rs` (depends on all modules)

**Coupled pair:** `pipeline.rs` <-> `vad.rs`

---

## 6. Summary of Actionable Items

| ID | Severity | Summary |
|---|---|---|
| ARCH-01 | High | `lib.rs` is a 1318-line God Module mixing commands, settings I/O, state types, and hotkey logic |
| ARCH-02 | High | Hotkey handler logic is duplicated between `setup()` and `rebind_hotkey()` (~130 lines identical) |
| ARCH-03 | Medium | `pipeline.rs` and `vad.rs` have bidirectional coupling |
| ARCH-04 | Medium | Backend and frontend use different access paths and key names for the same settings.json |
| ARCH-05 | Low | `models_dir()` duplicated across `download.rs` and `transcribe.rs` |
| ARCH-06 | Low | All inter-module deps go through Tauri managed state (implicit, runtime-resolved) |
| ARCH-07 | Low | Frontend prop-drilling is adequate for current size but won't scale |
| ARCH-08 | -- | IPC boundary is clean and well-structured (positive) |
| ARCH-09 | Low | Download types/utils duplicated between `FirstRun.tsx` and `ModelSelector.tsx` |
| ARCH-10 | Medium | Audio capture failure leaves managed state unregistered, causing panics |
