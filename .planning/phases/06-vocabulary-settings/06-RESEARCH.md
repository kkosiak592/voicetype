# Phase 6: Vocabulary + Settings - Research

**Researched:** 2026-02-28
**Domain:** Rust post-processing corrections, whisper-rs initial_prompt, cpal device enumeration, Tauri sidebar UI
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Correction Dictionary Editor**
- Inline two-column table (From / To) directly in the settings panel
- Add and delete rows in-place
- Whole word matching only — no substring matching (prevents "mpa" → "MPa" from corrupting "compare")
- Case-insensitive matching — handles whisper's variable capitalization; replacement uses exact casing from the "To" column
- Multi-word phrases supported on both From and To sides — critical for engineering terms ("why section" → "W-section", "aci three eighteen" → "ACI 318", "pounds per square inch" → "PSI")

**Profile System**
- Two shipped profiles only for v1: Structural Engineering and General
- Custom profile creation deferred to v2
- Profile switching takes effect immediately — next dictation uses the new profile's initial_prompt, corrections, and formatting
- Each profile has its own separate corrections dictionary — no shared global dictionary
- Each profile bundles: whisper initial_prompt, corrections dictionary, ALL CAPS output flag

**Settings Panel Layout**
- Sidebar navigation instead of single scrollable column
- No scrolling within content area — each section fits its view
- Settings window size increased from current dimensions (480×400) to accommodate sidebar + content
- Section grouping: Claude's discretion

**Model Selection**
- Curated list of 3 known models: large-v3-turbo (GPU, best accuracy), medium (balanced), small.en (CPU/fast)
- Each model shows name + description + recommended badge based on detected GPU hardware
- If a model file isn't downloaded yet, show it greyed out with download hint
- Model reload timing on selection change: Claude's discretion

**Microphone Selection**
- Dropdown listing available input devices by OS-reported name
- Default option = system default microphone
- Immediate switch — audio stream restarts with selected device on change, no app restart needed
- Selected device persists across restarts

### Claude's Discretion
- Profile switcher UI component design
- Settings sidebar section grouping and naming
- Settings window dimensions
- Model reload behavior (immediate vs restart)
- Loading/transition states during device and model switches
- Dictionary table styling and empty state

### Deferred Ideas (OUT OF SCOPE)
- Custom profile creation/deletion — v2 feature (PROF-01)
- Per-app profile auto-switching — v2 feature (PROF-02)
- Regex-based corrections for phonetic patterns — v2 feature (ECOR-01)
- Quick-add to dictionary from system tray — v2 feature (ECOR-03)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| VOC-01 | App applies a user-editable word correction dictionary (JSON find-and-replace) after each transcription | HashMap corrections engine in `corrections.rs`, applied in `pipeline.rs` between trim and inject |
| VOC-02 | User can create and switch between vocabulary profiles (each profile bundles a whisper initial_prompt + correction dictionary + output formatting) | Profile struct in `profiles.rs` with `ActiveProfile` managed state; Tauri command for switching |
| VOC-03 | App ships with a pre-configured "Structural Engineering" profile (I-beam, W-section, MPa, rebar, AISC, ACI 318, kips, PSI, prestressed) | Hard-coded in `profiles.rs` with `initial_prompt` string listing engineering terms |
| VOC-04 | App ships with a "General" profile (no domain bias, default corrections only) | Hard-coded in `profiles.rs` with empty initial_prompt and empty corrections |
| VOC-05 | User can enable ALL CAPS output mode per profile (for engineering drawing annotations and PDF markups) | `all_caps: bool` field on Profile struct; applied in pipeline after corrections, before inject |
| VOC-06 | Whisper initial_prompt is set per profile to bias the model toward domain-specific vocabulary | `params.set_initial_prompt()` in `transcribe_audio()` — accepts `&str`, verified in whisper-rs 0.15 docs |
| SET-01 | App has a settings panel UI for configuring hotkeys, model, microphone, profiles, and corrections | Full sidebar-based Settings.tsx rebuild; Tauri window config width/height increase |
| SET-03 | User can select which whisper model to use (large-v3-turbo for GPU, small for CPU, medium as alternative) | Model selection Tauri command reloads WhisperState with new context; model file presence checked |
| SET-04 | User can select which microphone to use from available input devices | `host.input_devices()` enumeration; stream restart via `start_persistent_stream_with_device()`; device name persisted |
</phase_requirements>

## Summary

Phase 6 has three distinct engineering tracks that build on each other: (1) a Rust corrections engine and profile system that augments the existing pipeline, (2) microphone and model selection that require refactoring `audio.rs` and the whisper loading path, and (3) a complete rebuild of the settings UI from flat scrollable list to a sidebar-navigation panel.

The backend work is straightforward Rust with no new crate dependencies. Corrections are a `HashMap<String, String>` applied with whole-word regex matching. Profile state is a new `Mutex<ActiveProfile>` managed state. The `set_initial_prompt()` method exists in whisper-rs 0.15 `FullParams` and accepts `&str` — already verified against official docs. Device enumeration uses `host.input_devices()` from `HostTrait`; the codebase already uses `device.description().name()` (cpal 0.17 pattern, confirmed in `audio.rs` line 110). Stream restart on device change means replacing the `AudioCapture` in managed state — a `Mutex<Option<AudioCapture>>` approach is the correct pattern.

The UI is the largest surface. The existing `App.tsx` is a flat 480×400 window that must become a wider sidebar-nav layout. All existing components (`HotkeyCapture`, `RecordingModeToggle`, `ThemeToggle`, `AutostartToggle`) remain unchanged — they slot into sidebar content panes. The `RecordingModeToggle.tsx` radio-card pattern (indigo selection state) is the model for the profile switcher. The `/frontend-design` skill governs all UI/UX work.

**Primary recommendation:** Implement in three sequential plans: corrections + profiles backend first (no UI changes), then mic/model selection backend + persistence, then the full settings UI that wires everything together. This minimizes risk — the pipeline works end-to-end before any UI changes.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| whisper-rs | 0.15 | `params.set_initial_prompt()` for vocabulary bias | Already in project; `FullParams::set_initial_prompt(&mut self, &str)` confirmed in 0.15 docs |
| cpal | 0.17 | `host.input_devices()` + `device.description().name()` | Already in project; `HostTrait::input_devices()` returns `InputDevices<Devices>` iterator |
| serde_json | 1 | Corrections dictionary serialization (HashMap → JSON) | Already in project; used throughout `lib.rs` for settings.json |
| tauri-plugin-store | 2 | Frontend persistence for active profile, selected mic, selected model | Already in project; `store.set()` / `store.get()` pattern established in `store.ts` |
| React + Tailwind CSS v4 | 18 / 4 | Settings sidebar UI | Already in project; all existing components use this stack |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| regex | 1 | Whole-word boundary matching for corrections (`\b` word boundary) | Corrections engine requires whole-word matching; stdlib `str::replace` does substring, not word-boundary |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| regex for word-boundary | Manual tokenization | Regex is simpler and handles multi-word phrases correctly; manual tokenization is error-prone for punctuation edge cases |
| Replacing AudioCapture in managed state | Rebuilding entire audio module | Replacement is safe if wrapped in `Mutex<Option<AudioCapture>>` — simpler than module rebuild |
| Hard-coded profiles in Rust | Profiles loaded from JSON files | Hard-coded is correct for v1 with only two profiles; JSON loading adds file-not-found error paths |

**Installation:**
```bash
# Add to Cargo.toml [dependencies]
regex = "1"
```

Note: No new npm packages required — all frontend work uses existing React/Tailwind/Tauri stack.

## Architecture Patterns

### Recommended Project Structure
```
src-tauri/src/
├── corrections.rs      # HashMap corrections engine, whole-word matching, apply_corrections()
├── profiles.rs         # Profile struct, STRUCTURAL_ENGINEERING + GENERAL constants, ActiveProfile state
├── audio.rs            # Refactored: start_persistent_stream_with_device(name: Option<String>)
├── transcribe.rs       # Add initial_prompt parameter to transcribe_audio()
├── lib.rs              # New Tauri commands + state registration for profiles, mic, model
└── pipeline.rs         # Call apply_corrections() + all_caps normalization before inject

src/
├── App.tsx             # Rebuilt as sidebar-nav layout shell
├── components/
│   ├── Sidebar.tsx         # Nav sidebar (sections list)
│   ├── sections/
│   │   ├── GeneralSection.tsx      # Hotkey + recording mode
│   │   ├── ProfilesSection.tsx     # Profile switcher + per-profile corrections editor
│   │   ├── ModelSection.tsx        # Model selector with GPU recommendation badge
│   │   ├── MicrophoneSection.tsx   # Mic dropdown
│   │   └── AppearanceSection.tsx   # Theme + autostart
│   ├── DictionaryEditor.tsx    # Two-column From/To table with add/delete rows
│   ├── ProfileSwitcher.tsx     # Radio-card pattern from RecordingModeToggle
│   ├── ModelSelector.tsx       # Model list with download state
│   └── [existing components unchanged]
└── lib/
    └── store.ts            # Extended: add activeProfile, selectedMic, selectedModel to AppSettings
```

### Pattern 1: Corrections Engine — Whole-Word HashMap Matching
**What:** Load `HashMap<String, String>` from JSON. Apply via regex with `\b` word boundaries around the "from" key.
**When to use:** After whisper transcription, before text injection. In `pipeline.rs` step 4.
**Example:**
```rust
// corrections.rs
use std::collections::HashMap;
use regex::Regex;

pub struct CorrectionsEngine {
    /// Each entry: (compiled regex with \b boundaries, replacement string)
    rules: Vec<(Regex, String)>,
}

impl CorrectionsEngine {
    pub fn from_map(map: &HashMap<String, String>) -> Result<Self, String> {
        let mut rules = Vec::new();
        for (from, to) in map {
            // Case-insensitive, whole-word boundary match
            let pattern = format!(r"(?i)\b{}\b", regex::escape(from));
            let re = Regex::new(&pattern).map_err(|e| e.to_string())?;
            rules.push((re, to.clone()));
        }
        Ok(Self { rules })
    }

    pub fn apply(&self, text: &str) -> String {
        let mut result = text.to_string();
        for (re, replacement) in &self.rules {
            result = re.replace_all(&result, replacement.as_str()).to_string();
        }
        result
    }
}
```

**Integration point in `pipeline.rs`** (between trim and inject, lines 142-150):
```rust
// After: let trimmed = transcription.trim_start();
// Before: let to_inject = format!("{} ", trimmed);

let profile = app.state::<crate::profiles::ActiveProfile>();
let profile_guard = profile.0.lock().unwrap();

// Apply corrections
let corrections_engine = app.state::<crate::corrections::CorrectionsState>();
let corrected = corrections_engine.apply(trimmed);

// Apply ALL CAPS if profile flag set
let formatted = if profile_guard.all_caps {
    corrected.to_uppercase()
} else {
    corrected
};

let to_inject = format!("{} ", formatted);
```

### Pattern 2: Profile State — Mutex-Backed Managed State
**What:** Active profile stored as `Mutex<Profile>` managed state. Switching profile replaces the inner value under lock.
**When to use:** Loaded at startup; updated by `set_active_profile` Tauri command.

```rust
// profiles.rs
use std::collections::HashMap;

#[derive(Clone)]
pub struct Profile {
    pub id: &'static str,
    pub name: &'static str,
    pub initial_prompt: &'static str,
    pub corrections: HashMap<String, String>,
    pub all_caps: bool,
}

pub static STRUCTURAL_ENGINEERING: Profile = Profile {
    id: "structural-engineering",
    name: "Structural Engineering",
    initial_prompt: "I-beam, W-section, W8x31, MPa, rebar, AISC, ACI 318, kips, PSI, \
                     prestressed concrete, shear wall, moment frame, deflection, \
                     compressive strength, tensile strength, grade 60 rebar",
    corrections: HashMap::new(), // populated at runtime with ::default_corrections()
    all_caps: false,
};

pub struct ActiveProfile(pub std::sync::Mutex<Profile>);
```

Note: `static` with `HashMap` doesn't work at compile time — profiles are constructed at runtime via a `build_profiles()` function. The `Profile` struct holds owned `HashMap<String, String>` populated from hard-coded arrays. See Pattern 3.

### Pattern 3: Profile Construction at Runtime (No static HashMap)
```rust
// profiles.rs — runtime construction pattern
pub fn structural_engineering_profile() -> Profile {
    let mut corrections = HashMap::new();
    corrections.insert("why section".to_string(), "W-section".to_string());
    corrections.insert("aci three eighteen".to_string(), "ACI 318".to_string());
    corrections.insert("pounds per square inch".to_string(), "PSI".to_string());
    corrections.insert("reinforcing bar".to_string(), "rebar".to_string());
    // ... more engineering corrections

    Profile {
        id: "structural-engineering",
        name: "Structural Engineering",
        initial_prompt: "I-beam, W-section, MPa, rebar, AISC, ACI 318, kips, PSI, \
                         prestressed concrete, shear wall, moment frame, deflection",
        corrections,
        all_caps: false,
    }
}

pub fn general_profile() -> Profile {
    Profile {
        id: "general",
        name: "General",
        initial_prompt: "",
        corrections: HashMap::new(),
        all_caps: false,
    }
}
```

### Pattern 4: whisper-rs initial_prompt Integration
**What:** `FullParams::set_initial_prompt(&mut self, initial_prompt: &str)` — verified in whisper-rs 0.15.
**When to use:** In `transcribe_audio()` before `state.full(params, audio)`.

```rust
// transcribe.rs — modify transcribe_audio() signature
pub fn transcribe_audio(
    ctx: &WhisperContext,
    audio: &[f32],
    initial_prompt: &str,  // NEW parameter
) -> Result<String, String> {
    // ...
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    // ... existing params ...
    if !initial_prompt.is_empty() {
        params.set_initial_prompt(initial_prompt);
    }
    // ...
}
```

**Whisper initial_prompt behavior (verified from official docs + whisper.cpp discussion #348):**
- Whisper treats the prompt as text it "just heard" before time zero — primes the decoder to expect similar vocabulary
- Fictitious prompts (just listing terms) are effective for terminology biasing
- Effective length: up to ~224 tokens (half the 448-token context window)
- Only affects the first 30-second segment (subsequent segments use previous output as context)
- For short dictation phrases (VoiceType's case), this covers 100% of use cases

### Pattern 5: Microphone Device Enumeration and Selection
**What:** List input devices by name; restart stream with selected device.
**When to use:** On settings load (populate dropdown) and on mic change.

```rust
// New Tauri command: list_input_devices
#[tauri::command]
fn list_input_devices() -> Result<Vec<String>, String> {
    use cpal::traits::HostTrait;
    let host = cpal::default_host();
    let devices = host.input_devices().map_err(|e| e.to_string())?;
    let mut names = vec!["System Default".to_string()];
    for device in devices {
        if let Ok(desc) = device.description() {
            names.push(desc.name().to_string());
        }
    }
    Ok(names)
}

// New Tauri command: set_microphone
#[tauri::command]
fn set_microphone(app: tauri::AppHandle, device_name: String) -> Result<(), String> {
    use cpal::traits::HostTrait;
    let host = cpal::default_host();

    let device = if device_name == "System Default" || device_name.is_empty() {
        host.default_input_device().ok_or("No default input device")?
    } else {
        host.input_devices()
            .map_err(|e| e.to_string())?
            .find(|d| {
                d.description()
                    .map(|desc| desc.name().to_string() == device_name)
                    .unwrap_or(false)
            })
            .ok_or_else(|| format!("Device '{}' not found", device_name))?
    };

    let new_capture = audio::start_persistent_stream_with_device(device)?;

    // Replace managed AudioCapture state
    // NOTE: Tauri managed state is immutable once set — use inner Mutex
    let audio_state = app.state::<audio::AudioCaptureMutex>();
    let mut guard = audio_state.0.lock().map_err(|e| e.to_string())?;
    *guard = new_capture;  // old stream drops here, new stream starts

    // Persist selection
    // ... serde_json write to settings.json (same pattern as set_recording_mode)

    Ok(())
}
```

**CRITICAL: AudioCapture managed state refactor required.** Current `AudioCapture` is registered with `app.manage(capture)` — once managed, Tauri state is immutable (only `&T` access). To replace the stream, wrap in `Mutex<AudioCapture>` before managing:

```rust
pub struct AudioCaptureMutex(pub std::sync::Mutex<AudioCapture>);
```

All existing code accessing `app.state::<audio::AudioCapture>()` must change to access via the Mutex guard. This is the primary refactoring risk in the phase — requires updating hotkey handler, pipeline, pill level stream, and stop/start commands.

### Pattern 6: Model Selection and Reload
**What:** User selects model → Tauri command loads new WhisperContext → replaces `WhisperState`.
**When to use:** On model change in settings UI.

The existing `WhisperState(pub Option<Arc<WhisperContext>>)` is `Option<Arc<...>>` — similar inner-Mutex approach needed for replacement:
```rust
pub struct WhisperStateMutex(pub std::sync::Mutex<Option<Arc<WhisperContext>>>);
```

Model file presence check (for greyed-out UI state):
```rust
#[tauri::command]
fn list_models(app: tauri::AppHandle) -> Vec<ModelInfo> {
    let has_gpu = /* check NVML at runtime, or store detection result in state */;
    vec![
        ModelInfo {
            id: "large-v3-turbo",
            name: "Large v3 Turbo",
            description: "Best accuracy, requires NVIDIA GPU",
            recommended: has_gpu,
            downloaded: transcribe::models_dir().join("ggml-large-v3-turbo-q5_0.bin").exists(),
        },
        ModelInfo {
            id: "medium",
            name: "Medium",
            description: "Balanced speed and accuracy",
            recommended: false,
            downloaded: transcribe::models_dir().join("ggml-medium.bin").exists(),
        },
        ModelInfo {
            id: "small-en",
            name: "Small (English)",
            description: "Fastest, works without GPU",
            recommended: !has_gpu,
            downloaded: transcribe::models_dir().join("ggml-small.en-q5_1.bin").exists(),
        },
    ]
}
```

**Model reload timing (Claude's discretion — recommendation):** Immediate on selection, blocking in `spawn_blocking`. Show loading state in UI. Fail gracefully if model file missing (keep previous model loaded).

### Pattern 7: Settings Sidebar UI — Layout Architecture
**What:** Replace flat `App.tsx` with sidebar-nav layout. Left sidebar = nav links; right content = active section.
**When to use:** Full settings panel rebuild for SET-01.

```tsx
// App.tsx rebuilt as sidebar layout
function App() {
  const [activeSection, setActiveSection] = useState<SectionId>('general');

  return (
    <div className="flex h-screen bg-white dark:bg-gray-900">
      {/* Sidebar nav */}
      <Sidebar activeSection={activeSection} onSelect={setActiveSection} />

      {/* Content pane */}
      <main className="flex-1 overflow-hidden p-6">
        {activeSection === 'general' && <GeneralSection />}
        {activeSection === 'profiles' && <ProfilesSection />}
        {activeSection === 'model' && <ModelSection />}
        {activeSection === 'microphone' && <MicrophoneSection />}
        {activeSection === 'appearance' && <AppearanceSection />}
      </main>
    </div>
  );
}
```

**Window size (Claude's discretion — recommendation):** Increase from 480×400 to 720×500. Sidebar ~180px wide leaves ~540px for content — enough for the dictionary table without scrolling.

**Section grouping (Claude's discretion — recommendation):**
- General: Hotkey, Recording Mode
- Profiles: Profile switcher + per-profile corrections + ALL CAPS toggle
- Model: Model selector
- Microphone: Mic dropdown
- Appearance: Theme, Autostart

### Anti-Patterns to Avoid
- **Substring corrections:** `str::replace` without word boundaries corrupts "mpa" → "MPa" inside words like "compare". Always use `\b` word boundaries via `regex` crate.
- **Tauri managed state replacement without Mutex:** `app.manage()` gives `&T` only — cannot move out or replace. Wrap in `Mutex` before managing.
- **Blocking audio thread with Mutex lock:** Existing pattern uses `try_lock()` in audio callback — maintain this. Never use `.lock()` (blocking) inside the cpal callback.
- **Setting initial_prompt on every call regardless:** If profile has empty initial_prompt (General profile), skip `set_initial_prompt()` call entirely — passing an empty string may still slightly affect decoding.
- **Loading whisper model on the async Tauri runtime thread:** Model loading is blocking/CPU-intensive — always use `spawn_blocking`.
- **Re-enumerating devices on every dropdown render:** Call `list_input_devices` once on settings open and cache the result in React state.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Whole-word boundary matching | Custom tokenizer / string split | `regex` crate with `\b` | Word boundaries in Unicode text are non-trivial; regex handles punctuation, hyphens correctly |
| Case-insensitive matching | `.to_lowercase()` comparison | `(?i)` flag in regex pattern | Handles Unicode case folding, avoids double-allocation |
| Settings persistence | Custom file I/O for profile/mic/model | Extend existing `settings.json` via `serde_json` (Rust) + `tauri-plugin-store` (frontend) | Already established pattern in codebase; both sides read the same file |
| Device enumeration | WinAPI calls directly | `cpal::HostTrait::input_devices()` | Already in project; cross-platform; WASAPI handled |

**Key insight:** The corrections engine is the only genuinely new Rust code. Everything else is extending existing patterns already established in the codebase.

## Common Pitfalls

### Pitfall 1: AudioCapture Managed State Is Immutable After manage()
**What goes wrong:** Calling `app.state::<AudioCapture>()` returns `State<'_, AudioCapture>` which derefs to `&AudioCapture`. You cannot replace it with a new stream after device switch.
**Why it happens:** Tauri's managed state is an `Arc<T>` under the hood — `manage()` locks in the type.
**How to avoid:** Wrap `AudioCapture` in a `Mutex` before registering: `app.manage(AudioCaptureMutex(Mutex::new(capture)))`. All access sites then do `app.state::<AudioCaptureMutex>().0.lock().unwrap()`.
**Warning signs:** Compiler error "cannot move out of `State<'_, AudioCapture>`" when trying to replace.

### Pitfall 2: WhisperState Replacement for Model Switching
**What goes wrong:** Same as AudioCapture — `WhisperState(pub Option<Arc<WhisperContext>>)` is immutable once managed.
**Why it happens:** Same Tauri managed state immutability constraint.
**How to avoid:** Either wrap in `Mutex` (same pattern as AudioCapture) or accept that model changes require app restart (simpler, but worse UX). Given the locked decision says "model reload timing: Claude's discretion," app restart is an acceptable implementation choice that avoids the Mutex refactor if prioritizing simplicity.
**Recommendation:** Use Mutex approach for consistency with AudioCapture refactor. Both need it.

### Pitfall 3: Corrections Engine on Hot Path — Compile Regex Once
**What goes wrong:** Compiling regex patterns inside `apply_corrections()` on every transcription adds latency.
**Why it happens:** `Regex::new()` is not cheap — it compiles the pattern.
**How to avoid:** Compile regexes once when corrections are loaded/changed, store `Vec<(Regex, String)>` in the engine. Rebuild the engine when the user saves dictionary changes.

### Pitfall 4: Multi-Word "From" Keys and Word Boundaries
**What goes wrong:** `\baci three eighteen\b` will not match at the end of a sentence before a period because `\b` checks for word-character transitions. "aci three eighteen." will match correctly (period is non-word char), but testing is needed for punctuation-heavy engineering text.
**Why it happens:** `\b` behavior with spaces in the middle of the pattern — the leading `\b` and trailing `\b` only apply to the start/end of the phrase.
**How to avoid:** This is actually fine for multi-word patterns — `\b` anchors the start and end of the phrase relative to non-word characters, which is the correct behavior. Test with `"aci three eighteen."` and `"aci three eighteen,"`.

### Pitfall 5: Corrections Dictionary Persistence — Per-Profile
**What goes wrong:** Saving corrections to a single top-level key in settings.json overwrites corrections for the other profile.
**Why it happens:** Each profile has its own separate corrections dictionary (locked decision).
**How to avoid:** Persist corrections under a profile-scoped key: `{ "corrections": { "structural-engineering": {...}, "general": {...} } }`.

### Pitfall 6: cpal Device Name Stability on Windows
**What goes wrong:** Device names on Windows (WASAPI) can include the audio driver version or change across driver updates. Persisting by name and looking up on next launch may fail to find the previously selected device.
**Why it happens:** WASAPI device names include the driver/hardware name which can change.
**How to avoid:** Fall back to system default when the saved device name is not found in the enumerated list. Log a warning. Do not error — silently use default.

### Pitfall 7: initial_prompt Token Budget
**What goes wrong:** Packing too many terms into the prompt pushes past ~224 tokens — whisper silently truncates.
**Why it happens:** Whisper's context window is 448 tokens; initial_prompt is limited to half (~224 tokens) in the whisper.cpp implementation.
**How to avoid:** Keep the structural engineering prompt concise — 15-20 key terms are sufficient. A comma-separated list of terms uses ~3-4 tokens each, so 20 terms ≈ 60-80 tokens — well within budget.

## Code Examples

Verified patterns from official sources:

### set_initial_prompt (whisper-rs 0.15 — verified from official docs)
```rust
// Source: https://docs.rs/whisper-rs/0.15.0/whisper_rs/struct.FullParams.html
// Signature: pub fn set_initial_prompt(&mut self, initial_prompt: &str)
// Panics if initial_prompt contains null bytes.

let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
if !initial_prompt.is_empty() {
    params.set_initial_prompt(initial_prompt);
}
```

### cpal Device Enumeration (cpal 0.17 — verified from official docs + existing audio.rs)
```rust
// Source: https://docs.rs/cpal/0.17.0/cpal/traits/trait.HostTrait.html
// Confirmed by existing audio.rs line 110: device.description().map(|d| d.name().to_string())

use cpal::traits::{DeviceTrait, HostTrait};

let host = cpal::default_host();
let input_devices: Vec<String> = host
    .input_devices()
    .map_err(|e| e.to_string())?
    .filter_map(|d| d.description().ok().map(|desc| desc.name().to_string()))
    .collect();
```

### tauri-plugin-store Rust access (verified from official README)
```rust
// Source: https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/store/README.md
use tauri_plugin_store::StoreExt;
use serde_json::json;

// In setup() or command handler:
let store = app.store("settings.json")?;
store.set("activeProfile".to_string(), json!("structural-engineering"));
let profile = store.get("activeProfile"); // Option<serde_json::Value>
```

### Sidebar UI Shell — TypeScript/React (pattern from existing RecordingModeToggle.tsx)
```tsx
// Pattern: indigo-500 border for selected state (matches existing RecordingModeToggle.tsx)
// Profile switcher uses same radio-card pattern
const isSelected = activeProfile === profile.id;
<button
  className={[
    'flex flex-1 flex-col rounded-lg border-2 px-3 py-2.5 text-left transition-colors',
    isSelected
      ? 'border-indigo-500 bg-indigo-50 dark:border-indigo-400 dark:bg-indigo-950'
      : 'border-gray-200 bg-white hover:border-gray-300 dark:border-gray-700 dark:bg-gray-800',
  ].join(' ')}
>
```

### Dictionary Editor — Two-Column Table Pattern
```tsx
// Inline editable table — no library needed, plain React controlled inputs
function DictionaryEditor({ corrections, onChange }: Props) {
  const [rows, setRows] = useState(
    Object.entries(corrections).map(([from, to]) => ({ from, to }))
  );

  function addRow() {
    setRows([...rows, { from: '', to: '' }]);
  }

  function deleteRow(i: number) {
    const next = rows.filter((_, idx) => idx !== i);
    setRows(next);
    onChange(Object.fromEntries(next.map(r => [r.from, r.to])));
  }

  return (
    <div>
      <table className="w-full text-sm">
        <thead>
          <tr>
            <th className="text-left text-xs font-medium uppercase tracking-wider text-gray-500 pb-2">From</th>
            <th className="text-left text-xs font-medium uppercase tracking-wider text-gray-500 pb-2">To</th>
            <th className="w-8" />
          </tr>
        </thead>
        <tbody className="space-y-1">
          {rows.map((row, i) => (
            <tr key={i}>
              <td><input value={row.from} onChange={...} className="w-full rounded border px-2 py-1" /></td>
              <td><input value={row.to} onChange={...} className="w-full rounded border px-2 py-1" /></td>
              <td><button onClick={() => deleteRow(i)}>×</button></td>
            </tr>
          ))}
        </tbody>
      </table>
      <button onClick={addRow}>+ Add entry</button>
    </div>
  );
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `device.name()` for cpal device name | `device.description().name()` | cpal 0.17.0 | `name()` deprecated; `description()` returns richer metadata including manufacturer |
| whisper.cpp hotwords (experimental) | `set_initial_prompt()` with term list | Always the stable approach | hotwords feature in whisper.cpp is experimental and not exposed in whisper-rs 0.15 |
| Scrollable single-column settings | Sidebar-nav layout | Phase 6 (this phase) | Required by locked decision; sidebar fits more sections without scroll |

**Deprecated/outdated:**
- `device.name()` (cpal): Use `device.description().name()` — already using correct API in `audio.rs`
- whisper.cpp `--hotwords` flag: Not available in whisper-rs 0.15 bindings; `set_initial_prompt()` is the stable alternative and equally effective for short dictation

## Open Questions

1. **Model reload: Mutex or restart?**
   - What we know: WhisperState is currently `Option<Arc<WhisperContext>>` — immutable once managed. Replacing requires Mutex wrapper.
   - What's unclear: Whether the latency of reloading a 500MB+ model (large-v3-turbo) mid-session is acceptable UX vs showing "restart required" message.
   - Recommendation: Wrap in `Mutex` for immediate reload using `spawn_blocking`. Show a loading spinner in the model selector during reload. This is consistent with the AudioCapture Mutex refactor that's already required for microphone switching.

2. **Corrections save timing — auto-save or explicit Save button?**
   - What we know: `tauri-plugin-store` has `autoSave: 100` (100ms debounce) on the frontend. Changes to a dictionary row fire on blur/change.
   - What's unclear: Whether row edits should auto-save immediately or require explicit "Save" action.
   - Recommendation: Auto-save on row blur (same as hotkey — no explicit save button in existing UI). Rebuild the CorrectionsEngine in managed state after save.

3. **Does `set_initial_prompt` work with whisper-rs 0.15 + whisper.cpp compiled with CUDA?**
   - What we know: `FullParams::set_initial_prompt(&mut self, &str)` is confirmed in official docs.
   - What's unclear: Whether there are any CUDA-specific constraints in whisper.cpp that affect initial_prompt handling. The method delegates to C FFI.
   - Recommendation: Test with the Structural Engineering profile after implementation. If initial_prompt has no effect, check that `params.set_no_context(true)` (currently set in `transcribe.rs:157`) doesn't conflict — `no_context` may suppress prompt usage. Consider removing `set_no_context(true)` when an initial_prompt is set.

   **IMPORTANT:** `set_no_context(true)` currently set in `transcribe.rs` line 157 may interfere with `set_initial_prompt()`. Per whisper.cpp source, `no_context` suppresses the use of previous-segment context AND initial prompt tokens. When a non-empty initial_prompt is set, `no_context` should be `false` (or not set). The General profile (empty prompt) can keep `no_context: true`.

## Sources

### Primary (HIGH confidence)
- `/tazz4843/whisper-rs` (Context7) — `FullParams`, `SamplingStrategy`, existing transcribe.rs patterns
- `https://docs.rs/whisper-rs/0.15.0/whisper_rs/struct.FullParams.html` — `set_initial_prompt(&mut self, &str)` method signature confirmed
- `/websites/rs_cpal_0_17_0_cpal` (Context7) — `HostTrait::input_devices()`, `DeviceTrait::description()` API
- `https://docs.rs/cpal/0.17.0/cpal/traits/trait.DeviceTrait.html` — `description()` return type, `name()` deprecation confirmed
- `/tauri-apps/plugins-workspace` (Context7) — `StoreExt`, `app.store()`, Rust-side store access pattern
- Existing codebase (`audio.rs:110`, `lib.rs`, `transcribe.rs`, `store.ts`) — established patterns verified by direct inspection

### Secondary (MEDIUM confidence)
- `https://github.com/ggml-org/whisper.cpp/discussions/348` — initial_prompt behavior, 224-token limit, fictitious prompts effectiveness
- `https://cookbook.openai.com/examples/whisper_prompting_guide` — prompt engineering for Whisper vocabulary bias
- `https://github.com/RustAudio/cpal/blob/master/examples/enumerate.rs` — device enumeration pattern with `description()`

### Tertiary (LOW confidence)
- WebSearch: cpal WASAPI device name stability on Windows driver updates — unverified, flagged as Pitfall 6

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries already in project; APIs verified against official docs
- Architecture: HIGH — patterns derived from existing codebase conventions + verified API signatures
- Pitfalls: HIGH (Pitfall 1-5) / LOW (Pitfall 6: device name stability — WebSearch only, unverified)
- `no_context` + `initial_prompt` conflict (Open Question 3): MEDIUM — derived from reading whisper.cpp source semantics, not directly confirmed by whisper-rs docs

**Research date:** 2026-02-28
**Valid until:** 2026-05-28 (stable libraries — 90 days)
