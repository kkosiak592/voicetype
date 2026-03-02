# Type Design Review: voice-to-text

**Date:** 2026-03-01
**Scope:** Full codebase -- Rust backend (`src-tauri/src/`) and TypeScript frontend (`src/`)
**Focus:** Invariant strength, type safety at IPC boundaries, discriminated unions, primitive overuse, encapsulation

---

## Summary

| Severity | Count | Description |
|----------|-------|-------------|
| Critical | 3 | Unsafe `as` casts at IPC boundary, pipeline state machine uses raw `u8`, duplicated `DownloadEvent` type without shared contract |
| High | 8 | Stringly-typed IPC commands, profile ID as bare `String`, model ID as bare `String`, `ProfileInfo` field mismatch across boundary, duplicated `model_info` mapping, `ModelInfo` duplicated in backend+frontend, unsafe `Sync` impls as workaround, pill event payloads are untyped strings |
| Medium | 10 | `Mode` enum lacks Serde, settings read functions are repetitive, `CorrectionsState`/`ActiveProfile` expose `pub` Mutex, `PipelineState::transition` accepts any `u8`, `RecordingMode` uses raw `AtomicU8`, `AppSettings` fields are plain strings, `store.get<T>` unsound generic, `FirstRunStatus` duplicated in Rust+TS, `write_wav` takes `&str` not `&Path`, `formatMB` duplicated |
| Low | 7 | `SidebarItem.icon` is string not enum, `HotkeyCapture.normalizeKey` returns bare string, missing branded types for audio sample rate, `DEFAULTS.selectedModel` empty string sentinel, `corrections` map key has no newtype, `level` is unbranded `f32`, repetitive settings JSON read/write pattern |

**Overall Assessment:** The codebase is well-structured for a v1 product. TypeScript `strict` mode is enabled and no `any` usage was found. The most impactful issues are at the IPC boundary where Rust and TypeScript types are maintained independently with no shared contract, creating drift risk. The pipeline state machine encodes states as raw `u8` constants rather than an enum with enforced transitions. Several domain concepts (model ID, profile ID, hotkey string) are represented as bare `String`/`string` when newtypes would prevent misuse.

---

## Detailed Findings

### CRITICAL

---

#### C1. Unsafe `as PillDisplayState` cast from untyped IPC event

**File:** `src/Pill.tsx:57`
```typescript
appWindow.listen<string>("pill-state", (e) => {
  if (e.payload === "idle") {
    return;
  }
  setDisplayState(e.payload as PillDisplayState);
});
```

**Problem:** The backend emits arbitrary strings (`"recording"`, `"processing"`, `"idle"`, `"error"`) via `app.emit_to("pill", "pill-state", ...)`. The frontend casts the payload directly to `PillDisplayState` with `as`, bypassing any runtime validation. If the backend ever emits an unexpected string (e.g., a new state added in Rust), the TypeScript type system silently accepts it. The `as` cast is a lie -- it tells the compiler "trust me" without proof.

**Backend emission sites (all use bare string literals):**
- `src-tauri/src/lib.rs:178` -- `app.emit_to("pill", "pill-state", "recording")`
- `src-tauri/src/lib.rs:236` -- `app.emit_to("pill", "pill-state", "processing")`
- `src-tauri/src/pipeline.rs:251` -- `app.emit_to("pill", "pill-state", "idle")`
- `src-tauri/src/vad.rs:256` -- `app.emit_to("pill", "pill-state", "idle")`
- `src-tauri/src/vad.rs:280` -- `app.emit_to("pill", "pill-state", "processing")`

**Recommendation:** Add a runtime guard before the cast:
```typescript
const VALID_STATES = new Set<PillDisplayState>(["recording", "processing", "error"]);
const state = e.payload;
if (VALID_STATES.has(state as PillDisplayState)) {
  setDisplayState(state as PillDisplayState);
}
```
And on the Rust side, define a `PillState` enum with Serde serialization so the emitted values are type-checked at compile time rather than being string literals scattered across files.

---

#### C2. Pipeline state machine uses raw `u8` constants instead of an enum

**File:** `src-tauri/src/pipeline.rs:6-8, 14-33`
```rust
pub const IDLE: u8 = 0;
pub const RECORDING: u8 = 1;
pub const PROCESSING: u8 = 2;

pub struct PipelineState(pub AtomicU8);

impl PipelineState {
    pub fn transition(&self, from: u8, to: u8) -> bool { ... }
    pub fn set(&self, val: u8) { ... }
}
```

**Problem:** The pipeline state machine is the most critical invariant in the system -- it prevents double-recordings, race conditions, and stuck states. Yet `transition()` and `set()` accept any `u8`, meaning `pipeline.set(255)` or `pipeline.transition(0, 42)` compiles without warning. There is no type-level guarantee that only valid states (0, 1, 2) are used. The `pub` inner field allows direct access to the `AtomicU8`, bypassing the transition guard entirely (`pipeline.0.store(99, ...)`).

**All call sites use the constants correctly today**, but this is enforced only by discipline:
- `src-tauri/src/pipeline.rs:245` -- `self.set(IDLE)`
- `src-tauri/src/vad.rs:239` -- `pipeline.transition(RECORDING, IDLE)`
- `src-tauri/src/vad.rs:264` -- `pipeline.transition(RECORDING, PROCESSING)`
- `src-tauri/src/lib.rs:167,193,223,254` -- various transitions

**Recommendation:** Replace with a proper enum and restrict the API:
```rust
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Phase { Idle = 0, Recording = 1, Processing = 2 }

pub struct PipelineState(AtomicU8); // private field

impl PipelineState {
    pub fn transition(&self, from: Phase, to: Phase) -> bool { ... }
    // Remove pub set() -- force all changes through transition()
}
```
This makes invalid states unrepresentable at the type level.

---

#### C3. `DownloadEvent` discriminated union duplicated across frontend and backend with no shared contract

**Rust definition** (`src-tauri/src/download.rs:16-38`):
```rust
#[derive(Clone, Serialize)]
#[serde(tag = "event", content = "data")]
pub enum DownloadEvent {
    #[serde(rename = "started")]
    Started { url: String, #[serde(rename = "totalBytes")] total_bytes: u64 },
    #[serde(rename = "progress")]
    Progress { #[serde(rename = "downloadedBytes")] downloaded_bytes: u64, ... },
    #[serde(rename = "finished")]
    Finished,
    #[serde(rename = "error")]
    Error { message: String },
}
```

**TypeScript definition** (`src/components/FirstRun.tsx:4-8` and duplicated at `src/components/ModelSelector.tsx:12-16`):
```typescript
type DownloadEvent =
  | { event: 'started'; data: { url: string; totalBytes: number } }
  | { event: 'progress'; data: { downloadedBytes: number; totalBytes: number } }
  | { event: 'finished' }
  | { event: 'error'; data: { message: string } };
```

**Problem:** Three-way drift risk. The TypeScript type is defined twice (FirstRun and ModelSelector) and must manually stay in sync with the Rust enum. If a field is renamed, added, or removed on either side, the compiler on neither side catches it. The Rust `#[serde(rename)]` annotations are the fragile link -- if someone changes `totalBytes` to `total_bytes` in TypeScript without updating Rust's rename, the frontend silently receives `undefined`.

**Recommendation:**
1. Extract the TypeScript `DownloadEvent` into a single shared file (e.g., `src/lib/types.ts`) and import from both components.
2. Consider generating TypeScript types from Rust structs using `ts-rs` or `specta` to maintain a single source of truth.

---

### HIGH

---

#### H1. Stringly-typed pill event system -- all pill IPC uses magic strings

**Files:** Throughout `src-tauri/src/pipeline.rs`, `src-tauri/src/vad.rs`, `src-tauri/src/lib.rs`, `src/Pill.tsx`

**Backend emissions (string literals, no type checking):**
```rust
// pipeline.rs:65
app.emit_to("pill", "pill-result", "error")
// pipeline.rs:200
app.emit_to("pill", "pill-result", "success")
// pipeline.rs:251
app.emit_to("pill", "pill-state", "idle")
// lib.rs:178
app.emit_to("pill", "pill-state", "recording")
```

**Frontend listeners (string matching):**
```typescript
// Pill.tsx:52
appWindow.listen<string>("pill-state", ...)
appWindow.listen<number>("pill-level", ...)
appWindow.listen<string>("pill-result", ...)
```

**Problem:** The event names `"pill-state"`, `"pill-result"`, `"pill-show"`, `"pill-hide"`, `"pill-level"` and their payloads are all untyped strings. There is no compile-time contract between sender and receiver. Adding a new event or changing a payload type requires grep-based coordination.

**Recommendation:** On the Rust side, create typed event enums:
```rust
enum PillEvent {
    Show,
    Hide,
    State(PillDisplayState),
    Level(f32),
    Result(PillResult),
}
```
Even without full codegen, a constants module shared between emit and listen call sites would reduce drift.

---

#### H2. Profile ID is bare `String` -- no newtype, no validation

**Files:**
- Rust: `src-tauri/src/profiles.rs:14` (`pub id: String`)
- Rust: `src-tauri/src/lib.rs:431` (`fn set_active_profile(... profile_id: String)`)
- Rust: `src-tauri/src/lib.rs:1037` (`match profile_id.as_str()`)
- TypeScript: `src/components/ProfileSwitcher.tsx:3` (`id: string`)
- TypeScript: `src/components/sections/ProfilesSection.tsx:8` (`activeProfileId: string`)

**Problem:** Profile IDs are compared and matched throughout the codebase (`"general"`, `"structural-engineering"`) but are represented as bare strings. The Rust `set_active_profile` command validates with a match statement, but any caller can pass arbitrary strings. The frontend `ProfileSwitcher` accepts any `string` as `id`. There is no single source of truth for valid profile IDs.

**Latent bug:** The profile ID `"structural_engineering"` in `PROFILE_DESCRIPTIONS` (TypeScript, `ProfileSwitcher.tsx:10`) uses underscores, but the Rust profile uses `"structural-engineering"` with hyphens (`profiles.rs:48`). The frontend description lookup silently fails, falling back to "Custom profile".

**Recommendation:** Create a `ProfileId` enum in Rust with Serde support. On the TypeScript side, use a union type `type ProfileId = "general" | "structural-engineering"`.

---

#### H3. Model ID is bare `String` with duplicated mapping logic

**Files:**
- `src-tauri/src/download.rs:51-65` (`fn model_info(model_id: &str)`)
- `src-tauri/src/lib.rs:657-665` (`fn model_id_to_path(model_id: &str)`)
- `src-tauri/src/lib.rs:686-702` (`fn list_models()`)
- `src-tauri/src/lib.rs:766-770` (model_id to ModelMode mapping in `set_model`)
- TypeScript: `src/components/FirstRun.tsx:18-33` (hardcoded `MODELS` array)

**Problem:** The concept of "model ID" appears in at least 5 separate locations with independent match/mapping logic. `download.rs` maps model IDs to `(filename, sha256, size)`. `lib.rs:model_id_to_path` maps to file paths. `lib.rs:list_models` maps to display info. `lib.rs:set_model` maps to `ModelMode`. `FirstRun.tsx` has its own hardcoded model list. None of these share a single source of truth. Adding a new model requires updating 5+ locations.

**Recommendation:** Create a `ModelId` enum in Rust and a single `ModelSpec` struct that contains all model metadata (filename, sha256, size, display name, description, mode). Then derive all mappings from that single registry. On the TypeScript side, fetch model metadata from the backend rather than hardcoding.

---

#### H4. `ProfileInfo` field name mismatch between backend and frontend -- actual bug

**Backend** (`src-tauri/src/lib.rs:402-407`):
```rust
#[derive(serde::Serialize)]
struct ProfileInfo {
    id: String,
    name: String,
    is_active: bool, // snake_case, no rename_all
}
```

**Frontend** (`src/components/ProfileSwitcher.tsx:3-7`):
```typescript
export interface ProfileInfo {
  id: string;
  name: string;
  active: boolean; // NOT is_active, NOT isActive
}
```

**Problem:** The Rust struct serializes `is_active` (snake_case, default serde behavior) but the TypeScript interface reads `active`. These do not match. The field `is_active` would serialize to `isActive` with camelCase renaming, still not `active`. This means `profile.active` is always `undefined` on the frontend. The profile selection UI works regardless because `ProfileSwitcher` uses the `activeId` prop for visual selection rather than the `active` field from the backend data. But the type contract is broken.

**Recommendation:** Add `#[serde(rename_all = "camelCase")]` to the Rust `ProfileInfo` struct and update the TypeScript interface to use `isActive: boolean`, OR rename the Rust field to `active` to match the frontend.

---

#### H5. `ModelInfo` type duplicated between backend and frontend with no contract

**Backend** (`src-tauri/src/lib.rs:669-676`):
```rust
struct ModelInfo {
    id: String,
    name: String,
    description: String,
    recommended: bool,
    downloaded: bool,
}
```

**Frontend** (`src/components/ModelSelector.tsx:4-10`):
```typescript
export interface ModelInfo {
  id: string;
  name: string;
  description: string;
  recommended: boolean;
  downloaded: boolean;
}
```

**Problem:** Same pattern as C3 -- manual duplication with drift risk. Currently matches because Rust field names are single-word lowercase, so default serde serialization aligns. Adding a multi-word field without `rename_all` would break silently.

---

#### H6. Unsafe `Sync` implementations on `AudioCapture` and `AudioCaptureMutex`

**File:** `src-tauri/src/audio.rs:91-106`
```rust
// SAFETY: cpal::Stream is Send but not Sync.
unsafe impl Sync for AudioCapture {}
// SAFETY: AudioCaptureMutex wraps AudioCapture (which is already Sync via unsafe impl).
unsafe impl Sync for AudioCaptureMutex {}
```

**Problem:** The `unsafe impl Sync` on `AudioCapture` is technically sound given the documented constraints (no shared `&cpal::Stream` across threads), but it circumvents the type system's protection. The second `unsafe impl Sync for AudioCaptureMutex` is redundant -- `Mutex<T>` is `Sync` when `T: Send`, and `AudioCapture` is `Send` (cpal::Stream is Send). The real issue is that the `_stream` field could be accidentally referenced across threads in future changes within the module.

**Recommendation:** Consider extracting `cpal::Stream` into a `Send`-only wrapper to make the `unsafe` more targeted, or accept it but ensure the module stays small enough that the safety argument remains verifiable.

---

#### H7. `FirstRunStatus` duplicated between Rust and TypeScript

**Backend** (`src-tauri/src/lib.rs:707-713`):
```rust
#[serde(rename_all = "camelCase")]
struct FirstRunStatus {
    needs_setup: bool,
    gpu_detected: bool,
    recommended_model: String,
}
```

**Frontend** (`src/App.tsx:12-16`):
```typescript
interface FirstRunStatus {
  needsSetup: boolean;
  gpuDetected: boolean;
  recommendedModel: string;
}
```

**Problem:** Same duplication pattern. This one uses `rename_all = "camelCase"` correctly so fields actually match. But drift risk remains. The frontend constructs a synthetic `FirstRunStatus` in the catch block (`App.tsx:36`) with `recommendedModel: ''`, which is an invalid model ID.

---

#### H8. Pill event payload strings have no type-level connection to backend emissions

**File:** `src/Pill.tsx:52-58`
```typescript
appWindow.listen<string>("pill-state", (e) => {
  if (e.payload === "idle") { return; }
  setDisplayState(e.payload as PillDisplayState);
});
```

**And:**
```typescript
appWindow.listen<string>("pill-result", () => { ... });
```

The generic `<string>` type parameter provides zero safety. Any payload type could be emitted from Rust. The `pill-result` listener declares `<string>` but ignores the payload entirely -- the type annotation is misleading documentation.

---

### MEDIUM

---

#### M1. `Mode` enum lacks Serde derivation -- IPC uses string mapping instead

**File:** `src-tauri/src/lib.rs:30-33, 112-131`
```rust
pub enum Mode {
    HoldToTalk = 0,
    Toggle = 1,
}

#[tauri::command]
fn set_recording_mode(app: tauri::AppHandle, mode: String) -> Result<(), String> {
    match mode.as_str() {
        "toggle" => recording_mode.set(Mode::Toggle),
        _ => recording_mode.set(Mode::HoldToTalk),
    }
```

**Problem:** `Mode` is a clean enum but the IPC command accepts `String` instead of `Mode` directly. The match arm `_ => Mode::HoldToTalk` silently accepts any garbage input. If Serde `Deserialize` were derived on `Mode`, Tauri would deserialize the enum variant directly, failing with a clear error on invalid input rather than silently defaulting.

---

#### M2. `PipelineState` inner field is `pub` -- invariant bypass possible

**File:** `src-tauri/src/pipeline.rs:14`
```rust
pub struct PipelineState(pub AtomicU8);
```

The `pub` on the inner `AtomicU8` allows any code with access to call `.0.store(...)`, bypassing the `transition()` method's CAS guard. This undermines the core safety invariant of the state machine.

---

#### M3. `RecordingMode` stores `Mode` as raw `AtomicU8` with catch-all

**File:** `src-tauri/src/lib.rs:39-54`
```rust
pub struct RecordingMode(pub std::sync::atomic::AtomicU8);

impl RecordingMode {
    pub fn get(&self) -> Mode {
        match self.0.load(...) {
            1 => Mode::Toggle,
            _ => Mode::HoldToTalk,
        }
    }
}
```

**Problem:** The `get()` method uses `_ => Mode::HoldToTalk` as a catch-all for any u8 value other than 1. If the `AtomicU8` somehow holds a corrupted value (e.g., via the `pub` inner field), the error is silently hidden.

---

#### M4. `CorrectionsState` and `ActiveProfile` expose `pub` Mutex

**Files:**
- `src-tauri/src/corrections.rs:75`: `pub struct CorrectionsState(pub std::sync::Mutex<CorrectionsEngine>);`
- `src-tauri/src/profiles.rs:84`: `pub struct ActiveProfile(pub std::sync::Mutex<Profile>);`

**Problem:** Exposing the inner `Mutex` as `pub` means any caller can lock it directly and mutate contents arbitrarily. There is no encapsulated API for "swap the engine" or "read the profile". All mutation goes through raw `.0.lock().unwrap()` patterns scattered across `lib.rs`. This makes it impossible to enforce invariants like "corrections engine must always match active profile's corrections map" at the type level.

---

#### M5. `store.get<T>()` casts are unsound -- TypeScript trusts the store blindly

**File:** `src/App.tsx:40-45`
```typescript
const savedHotkey = await store.get<string>('hotkey');
const savedTheme = await store.get<'light' | 'dark'>('theme');
const savedRecordingMode = await store.get<'hold' | 'toggle'>('recordingMode');
```

**Problem:** `store.get<T>()` from `@tauri-apps/plugin-store` returns `T | null`, but the generic `T` is purely a cast -- the store contains `unknown` JSON values. If the settings file is manually edited to `"theme": 42`, the code receives `42` typed as `'light' | 'dark'`. No runtime validation exists.

**Recommendation:** Add runtime checks after retrieval:
```typescript
const rawTheme = await store.get<string>('theme');
const savedTheme = rawTheme === 'light' || rawTheme === 'dark' ? rawTheme : null;
```

---

#### M6. `AppSettings` fields use plain strings where constrained types exist

**File:** `src/lib/store.ts:3-11`
```typescript
export interface AppSettings {
  hotkey: string;          // Should be branded/validated
  theme: 'light' | 'dark'; // Good -- uses union
  autostart: boolean;
  recordingMode: 'hold' | 'toggle'; // Good -- uses union
  activeProfile: string;   // Should be ProfileId union
  selectedMic: string;     // Could be branded
  selectedModel: string;   // Should match model ID union
}
```

**Problem:** `hotkey`, `activeProfile`, and `selectedModel` are bare `string` types. `DEFAULTS.selectedModel` is empty string `''` which is a sentinel value rather than a proper "no selection" representation (`null` or `undefined`).

---

#### M7. `write_wav` takes `&str` instead of `&Path`

**File:** `src-tauri/src/audio.rs:209`
```rust
pub fn write_wav(path: &str, samples: &[f32]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
```

Accepts `&str` rather than `&std::path::Path`. All callers already have a `PathBuf` and call `.to_string_lossy()` before passing it in (`lib.rs:317`).

---

#### M8. `formatMB` utility duplicated

**Files:**
- `src/components/FirstRun.tsx:35-37`
- `src/components/ModelSelector.tsx:26-28`

Both define identical `formatMB(bytes: number): string` functions. Should be extracted to a shared utility.

---

#### M9. Settings JSON read functions are repetitive and error-prone

**File:** `src-tauri/src/lib.rs:75-86, 90-110, 331-350, 354-377, 381-399, 563-572`

Six separate functions (`read_saved_hotkey`, `read_saved_mode`, `read_saved_profile_id`, `read_saved_corrections`, `read_saved_all_caps`, `read_saved_mic`) all follow the identical pattern:
1. Get app data dir
2. Build settings path
3. Read file
4. Parse JSON
5. Extract field

Each function independently opens, reads, and parses the same file. No shared abstraction exists. Adding a new setting requires copying the entire boilerplate. The JSON keys (`"hotkey"`, `"recording_mode"`, `"active_profile_id"`, `"microphone_device"`, `"whisper_model_id"`) are string literals with no shared constant.

---

#### M10. `DEFAULTS` cast uses `as unknown as Record<string, unknown>`

**File:** `src/lib/store.ts:28`
```typescript
defaults: DEFAULTS as unknown as Record<string, unknown>,
```

Double cast through `unknown` -- a type safety escape hatch.

---

### LOW

---

#### L1. `SidebarItem.icon` is string instead of constrained type

**File:** `src/components/Sidebar.tsx:1-7`
```typescript
interface SidebarItem {
  id: SectionId;
  label: string;
  icon: string; // Unicode characters
}
```

Not a significant risk -- the icons are hardcoded in a const array.

---

#### L2. `HotkeyCapture.normalizeKey` returns nullable string with no branded type

**File:** `src/components/HotkeyCapture.tsx:10-61`

Returns `string | null` where the string represents a `modifier+modifier+key` format. A branded type would prevent accidental use of arbitrary strings where hotkey combos are expected.

---

#### L3. Audio sample rate (16000) appears as magic number throughout

**Files:**
- `src-tauri/src/audio.rs:20-23` -- resampler target rate
- `src-tauri/src/audio.rs:210` -- WAV spec
- `src-tauri/src/pipeline.rs:77,87` -- duration calculation
- `src-tauri/src/vad.rs:43` -- VAD sample rate

The value `16000` (or `16_000`) is used as a raw integer in many places. A named constant `const WHISPER_SAMPLE_RATE: u32 = 16_000;` would document the invariant.

---

#### L4. `level` prop uses raw `number`/`f32` with no documented range enforcement

**Files:**
- `src/components/FrequencyBars.tsx:3-4`: `level: number; // 0.0 - 1.0`
- `src-tauri/src/pill.rs:94-101`: `fn compute_rms(...) -> f32`

The comment documents the 0.0-1.0 range but the type does not enforce it.

---

#### L5. `DEFAULTS.selectedModel` uses empty string as sentinel

**File:** `src/lib/store.ts:20`
```typescript
selectedModel: '',
```

`App.tsx:62` has a special check `if (savedSelectedModel !== null && savedSelectedModel !== undefined)` indicating the type does not properly represent "no selection."

---

#### L6. `models_dir()` duplicated between `transcribe.rs` and `download.rs`

**Files:** `src-tauri/src/transcribe.rs:16-18`, `src-tauri/src/download.rs:44-47`

Identical function body. Comment in `download.rs` acknowledges the duplication to avoid feature-gate coupling.

---

#### L7. Settings JSON key names diverge between frontend and backend

**Problem:** The backend reads/writes `settings.json` directly with `serde_json` using keys like `"recording_mode"`, `"active_profile_id"`, `"microphone_device"`. The frontend uses `@tauri-apps/plugin-store` with keys like `"recordingMode"`, `"activeProfile"`, `"selectedMic"`. These are **different key names** for the same concepts writing to the **same file**:

| Concept | Backend key | Frontend key |
|---------|------------|--------------|
| Recording mode | `recording_mode` | `recordingMode` |
| Active profile | `active_profile_id` | `activeProfile` |
| Microphone | `microphone_device` | `selectedMic` |
| Model | `whisper_model_id` | `selectedModel` |

Each side reads only its own keys and ignores the other's. Changing recording mode in the UI writes `"recordingMode"` but the backend's `read_saved_mode()` reads `"recording_mode"` -- the backend never sees the frontend's change on restart.

---

## Positive Observations

1. **TypeScript `strict` mode is enabled** (`tsconfig.json:19`) with `noUnusedLocals`, `noUnusedParameters`, and `noFallthroughCasesInSwitch`. No `any` usage found anywhere.

2. **`SectionId` is a proper union type** (`Sidebar.tsx:1`): `type SectionId = 'general' | 'profiles' | 'model' | 'microphone' | 'appearance'`. Used correctly throughout.

3. **`PillDisplayState` and `AnimState` are union types** (`Pill.tsx:8-9`), giving compile-time exhaustiveness for pill UI states.

4. **`DownloadEvent` uses proper discriminated union** in TypeScript with `event` as the discriminant. The `switch (msg.event)` gets exhaustiveness checking.

5. **`TrayState` enum** (`tray.rs:12-16`) is clean -- simple enum with clear variants, matched exhaustively.

6. **`DownloadEvent` Rust enum** (`download.rs:16-38`) uses proper tagged enum serialization with `#[serde(tag = "event", content = "data")]`.

7. **`CorrectionsEngine`** (`corrections.rs:25-68`) is well-encapsulated -- the `Rule` struct is private, construction goes through `from_map()` which validates all inputs, and `apply()` is the only public operation.

8. **`ResamplingState` struct** (`audio.rs:11-73`) is private to the module with a clear invariant.

9. **Component prop types are consistently defined** with explicit interfaces throughout React components.

10. **`Mode` enum** in Rust uses named variants rather than booleans for recording mode.

---

## Prioritized Recommendations

### Tier 1 -- High impact, moderate effort

1. **Fix the `ProfileInfo.is_active` / `active` field mismatch** (H4). This is an actual bug. Add `#[serde(rename_all = "camelCase")]` or rename the field.

2. **Fix the profile ID mismatch** (H2). TypeScript `PROFILE_DESCRIPTIONS` uses `structural_engineering` (underscore) but Rust uses `structural-engineering` (hyphen). Description lookup silently fails.

3. **Fix settings key name divergence** (L7). The backend and frontend are writing/reading different keys for the same settings. This means backend cannot restore frontend-saved settings on restart. Either standardize on one set of key names or have both sides use the store plugin.

4. **Create a Rust `PillState` enum** to replace string literals in `emit_to` calls (H1, C1). This prevents typos and enables exhaustive matching.

5. **Make `PipelineState` inner field private** and change `transition()`/`set()` to accept a `Phase` enum (C2, M2). Highest safety ROI.

### Tier 2 -- Medium impact, lower effort

6. **Extract `DownloadEvent` to a shared TypeScript file** (C3, M8). Single import, both FirstRun and ModelSelector use it. Extract `formatMB` at the same time.

7. **Add `#[serde(rename_all = "camelCase")]` to all IPC structs** that lack it (H5). Prevents future field-naming bugs.

8. **Create a `ModelId` enum or registry** (H3). Centralizes the 5+ model-mapping locations.

9. **Add runtime validation for `store.get<T>()`** results (M5). At least for theme and recording mode.

### Tier 3 -- Lower priority, nice-to-have

10. **Make `CorrectionsState` and `ActiveProfile` inner Mutex private** (M4). Add accessor methods.

11. **Define `const WHISPER_SAMPLE_RATE: u32 = 16_000`** and use throughout (L3).

12. **Add Serde derives to `Mode` enum** so IPC commands can deserialize it directly (M1).

13. **Consider `ts-rs` or `specta`** for auto-generating TypeScript types from Rust structs to eliminate all manual IPC type duplication.
