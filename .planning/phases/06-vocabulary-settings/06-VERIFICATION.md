---
phase: 06-vocabulary-settings
verified: 2026-02-28T00:00:00Z
status: passed
score: 13/13 must-haves verified
re_verification: true
gaps:
  - truth: "Corrections dictionary saves user edits and persists to disk"
    status: resolved
    reason: "save_corrections Tauri command parameter name mismatch: frontend sends { corrections: updated } but Rust fn parameter is corrections_map. Tauri 2 camelCase-to-snake_case conversion maps 'corrections' -> 'corrections', not 'corrections_map'. The deserializer will fail to populate corrections_map, resulting in an empty map being saved to disk. In-session corrections appear to work because get_corrections returns in-memory state, masking the save failure."
    artifacts:
      - path: "src-tauri/src/lib.rs"
        issue: "save_corrections fn parameter is corrections_map but frontend sends key 'corrections'"
      - path: "src/components/sections/ProfilesSection.tsx"
        issue: "invoke('save_corrections', { corrections: updated }) — key 'corrections' does not match Rust parameter 'corrections_map'"
    missing:
      - "Rename Rust parameter: fn save_corrections(... corrections: HashMap<String, String>) — OR update frontend to send { correctionsMap: updated }"
human_verification:
  - test: "Corrections dictionary persistence across restart"
    expected: "Add a correction entry, close and reopen app, open Profiles section — correction entry must still appear"
    why_human: "The save_corrections parameter mismatch means disk writes may silently fail. Human must restart app and verify corrections survive the restart."
  - test: "Structural Engineering profile accuracy improvement"
    expected: "Dictate 'why section W8x31 ACI 318' with General profile active, then switch to Structural Engineering and repeat — SE profile should produce better transcription of engineering terms"
    why_human: "Cannot verify whisper bias behavior programmatically — requires actual audio input and transcription comparison."
  - test: "Model selector shows correct download status"
    expected: "Open Model section, verify each model card shows downloaded/not-downloaded state correctly based on files in %APPDATA%/VoiceType/models/"
    why_human: "Requires running app to call list_models and verify runtime file-existence checks."
---

# Phase 6: Vocabulary Settings — Verification Report

**Phase Goal:** Word correction dictionary, vocabulary profiles with engineering and general presets, and a full settings panel — the differentiating layer that makes VoiceType accurate for structural engineering work
**Verified:** 2026-02-28
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|---------|
| 1 | Corrections engine applies whole-word case-insensitive replacements to transcription text | VERIFIED | corrections.rs line 43: `(?i)\b{escaped_from}\b` regex, apply() iterates rules — 8 behavior tests in corrections_tests.rs confirm |
| 2 | Multi-word phrases are matched and replaced correctly | VERIFIED | corrections_tests.rs test `multi_word_phrase_replacement`: "aci three eighteen is the code" -> "ACI 318 is the code" |
| 3 | Structural Engineering profile biases whisper via initial_prompt | VERIFIED | profiles.rs structural_engineering_profile() returns non-empty initial_prompt with "I-beam, W-section, W8x31, MPa..." — transcribe.rs line 167-169: if !initial_prompt.is_empty() { params.set_initial_prompt(...); params.set_no_context(false); } |
| 4 | General profile has empty initial_prompt and no corrections | VERIFIED | profiles.rs general_profile(): empty String::new(), HashMap::new(), all_caps: false — confirmed by test general_profile_fields |
| 5 | ALL CAPS flag on a profile uppercases all injected text | VERIFIED | pipeline.rs line 176-179: if guard.all_caps { corrected.to_uppercase() } else { corrected } — applied before format!("{} ", formatted) |
| 6 | Profile switching changes corrections dictionary and initial_prompt | VERIFIED | lib.rs set_active_profile(): rebuilds new Profile from profile_id, merges user corrections, rebuilds CorrectionsEngine, updates both ActiveProfile and CorrectionsState managed states |
| 7 | set_no_context is disabled when initial_prompt is non-empty | VERIFIED | transcribe.rs line 167-172: `if !initial_prompt.is_empty() { params.set_initial_prompt(initial_prompt); params.set_no_context(false); } else { params.set_no_context(true); }` |
| 8 | User can list available microphone input devices | VERIFIED | lib.rs list_input_devices() command registered in invoke_handler (line 940), returns ["System Default", ...device names] |
| 9 | User can switch microphone and audio stream restarts | VERIFIED | lib.rs set_microphone() calls audio::start_persistent_stream_with_device(device), then replaces inner AudioCapture inside AudioCaptureMutex lock |
| 10 | User can list and switch whisper models without app restart | VERIFIED | lib.rs list_models() returns 3 ModelInfo entries with download status, set_model() uses spawn_blocking to load new WhisperContext and replaces WhisperStateMutex inner value |
| 11 | Settings panel has sidebar navigation with five sections | VERIFIED | App.tsx renders `<Sidebar activeSection={activeSection} onSelect={setActiveSection} />` with five conditionals (general/profiles/model/microphone/appearance). Sidebar.tsx renders five nav items |
| 12 | DictionaryEditor shows inline From/To table with add/delete | VERIFIED | DictionaryEditor.tsx: renders `<table>` with From/To columns, handleAdd appends empty row, handleDelete filters row out, handleBlur calls onChange triggering save_corrections |
| 13 | Corrections dictionary saves user edits and persists to disk | FAILED | Frontend sends `invoke('save_corrections', { corrections: updated })` but Rust fn signature is `fn save_corrections(..., corrections_map: HashMap<...>)`. Tauri 2 maps JS key 'corrections' to Rust param 'corrections', not 'corrections_map'. Deserialization will populate corrections_map with empty map. Corrections work in-session (in-memory state) but are NOT persisted to disk. |

**Score:** 12/13 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src-tauri/src/corrections.rs` | HashMap-backed corrections engine with regex word-boundary matching | VERIFIED | 76 lines, CorrectionsEngine::from_map + apply, CorrectionsState mutex wrapper — substantive |
| `src-tauri/src/profiles.rs` | Profile struct, built-in profiles, ActiveProfile managed state | VERIFIED | 85 lines, structural_engineering_profile (13 corrections, full initial_prompt), general_profile, get_all_profiles, ActiveProfile — substantive |
| `src-tauri/src/corrections_tests.rs` | 8 behavior tests | VERIFIED | 93 lines, 8 #[test] functions covering whole-word match, multi-word phrase, empty map, profile fields, ALL CAPS |
| `src-tauri/src/pipeline.rs` | Corrections + ALL CAPS applied between trim and inject | VERIFIED | Lines 169-182: corrections engine applied after trim_start, ALL CAPS applied after corrections, format!("{} ", formatted) |
| `src-tauri/src/transcribe.rs` | initial_prompt parameter threaded through to whisper FullParams | VERIFIED | Line 149: `pub fn transcribe_audio(ctx: &WhisperContext, audio: &[f32], initial_prompt: &str)`, lines 167-172: conditional set_initial_prompt + set_no_context |
| `src-tauri/src/audio.rs` | AudioCaptureMutex wrapper, start_persistent_stream_with_device() | VERIFIED | Lines 102-206: AudioCaptureMutex struct, build_stream_from_device private fn, start_persistent_stream_with_device public fn |
| `src-tauri/src/lib.rs` | All Tauri commands registered: get_profiles, set_active_profile, get_corrections, save_corrections, set_all_caps, list_input_devices, set_microphone, list_models, set_model | VERIFIED | invoke_handler lines 933-955: all 9 commands registered. WhisperStateMutex defined line 70 |
| `src-tauri/tauri.conf.json` | Settings window 720x500 | VERIFIED | width: 720, height: 500 — confirmed |
| `src/lib/store.ts` | AppSettings extended with activeProfile, selectedMic, selectedModel | VERIFIED | Lines 8-10: all three fields in interface, lines 18-20: defaults set (general, System Default, '') |
| `src/App.tsx` | Sidebar-nav layout shell replacing flat scrollable column | VERIFIED | flex h-screen layout, Sidebar component, five section conditionals, overflow-y-auto on main |
| `src/components/Sidebar.tsx` | Navigation sidebar with five section links | VERIFIED | 47 lines, five ITEMS, indigo-50/indigo-600 active state, SectionId type exported |
| `src/components/sections/GeneralSection.tsx` | Hotkey + Recording Mode settings section | VERIFIED | 50 lines, HotkeyCapture + RecordingModeToggle components — substantive |
| `src/components/sections/ProfilesSection.tsx` | Profile switcher + corrections editor + ALL CAPS toggle | VERIFIED | 120 lines, ProfileSwitcher, DictionaryEditor, ALL CAPS toggle, invoke calls for get_profiles/set_active_profile/get_corrections/set_all_caps/save_corrections |
| `src/components/sections/ModelSection.tsx` | Model selector with GPU recommendation | VERIFIED | 48 lines, invokes list_models, ModelSelector component, invokes set_model |
| `src/components/sections/MicrophoneSection.tsx` | Microphone dropdown selector | VERIFIED | 61 lines, invokes list_input_devices, select element, invokes set_microphone |
| `src/components/sections/AppearanceSection.tsx` | Theme toggle + autostart toggle | VERIFIED | 53 lines, ThemeToggle + AutostartToggle components |
| `src/components/DictionaryEditor.tsx` | Inline two-column From/To table with add/delete | VERIFIED | 132 lines, table with From/To columns, add/delete handlers, blur auto-save, empty state message |
| `src/components/ProfileSwitcher.tsx` | Radio-card profile selection component | VERIFIED | 61 lines, border-2 rounded-lg, indigo-500/indigo-50 selected state matching RecordingModeToggle |
| `src/components/ModelSelector.tsx` | Model list with download status and recommended badge | VERIFIED | 92 lines, per-card loading indicator, opacity-50 for not-downloaded, Recommended badge |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| pipeline.rs | corrections.rs (CorrectionsState) | app.state::\<crate::corrections::CorrectionsState\>() | VERIFIED | Line 170: `let engine = app.state::<crate::corrections::CorrectionsState>();` then `guard.apply(trimmed)` |
| pipeline.rs | profiles.rs (ActiveProfile) | app.state::\<crate::profiles::ActiveProfile\>() | VERIFIED | Line 177: `let profile = app.state::<crate::profiles::ActiveProfile>();` then `if guard.all_caps { corrected.to_uppercase() }` |
| pipeline.rs | transcribe.rs | initial_prompt passed to transcribe_audio() | VERIFIED | Line 96-100: initial_prompt cloned from ActiveProfile before spawn_blocking; line 121: `crate::transcribe::transcribe_audio(&ctx, &samples, &initial_prompt)` |
| lib.rs (set_microphone) | audio.rs (start_persistent_stream_with_device) | Replaces AudioCapture inside AudioCaptureMutex | VERIFIED | Line 613: `audio::start_persistent_stream_with_device(device)`, line 619: `*guard = new_capture` |
| lib.rs (set_model) | transcribe.rs (load_whisper_context) | Replaces WhisperContext inside WhisperStateMutex | VERIFIED | Line 735: `crate::transcribe::load_whisper_context(&path_str, &mode)`, line 745: `*guard = Some(Arc::new(new_ctx))` |
| pipeline.rs | audio.rs (AudioCaptureMutex) | All AudioCapture access through Mutex guard | VERIFIED | Lines 53-58: `app.state::<crate::audio::AudioCaptureMutex>()`, `.0.lock().unwrap()` then flush/get |
| ProfilesSection.tsx | Tauri commands (get_profiles, set_active_profile, get_corrections, save_corrections, set_all_caps) | invoke() calls | PARTIAL | All five commands invoked. However save_corrections has a parameter name mismatch (see gap below) |
| ModelSection.tsx | Tauri commands (list_models, set_model) | invoke() calls | VERIFIED | Lines 17, 28: `invoke<ModelInfo[]>('list_models')`, `invoke('set_model', { modelId })` |
| MicrophoneSection.tsx | Tauri commands (list_input_devices, set_microphone) | invoke() calls | VERIFIED | Lines 16, 27: `invoke<string[]>('list_input_devices')`, `invoke('set_microphone', { deviceName })` |
| store.ts | tauri-plugin-store settings.json | Extended AppSettings with activeProfile, selectedMic, selectedModel | VERIFIED | store.ts lines 8-10: fields declared; App.tsx useEffect reads all three from store |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|---------|
| VOC-01 | 06-01 | User-editable word correction dictionary applied after transcription | SATISFIED | CorrectionsEngine in corrections.rs, applied in pipeline.rs, DictionaryEditor UI in ProfilesSection. Note: saving to disk has parameter mismatch (see gap) |
| VOC-02 | 06-01 | Vocabulary profiles with whisper initial_prompt + correction dict + output formatting | SATISFIED | profiles.rs Profile struct with initial_prompt + corrections + all_caps fields, ProfileSwitcher UI, set_active_profile command |
| VOC-03 | 06-01 | Pre-configured Structural Engineering profile | SATISFIED | profiles.rs structural_engineering_profile(): 13 corrections, initial_prompt with "I-beam, W-section, MPa..." — confirmed by test |
| VOC-04 | 06-01 | General profile with no domain bias | SATISFIED | profiles.rs general_profile(): empty initial_prompt, empty corrections — confirmed by test |
| VOC-05 | 06-01 | ALL CAPS output mode per profile | SATISFIED | Pipeline applies to_uppercase() when guard.all_caps, set_all_caps Tauri command, UI toggle in ProfilesSection |
| VOC-06 | 06-01 | Whisper initial_prompt set per profile to bias model | SATISFIED | transcribe.rs accepts initial_prompt parameter, set_initial_prompt + set_no_context(false) when non-empty, pipeline reads from ActiveProfile before spawn_blocking |
| SET-01 | 06-03 | Settings panel UI for all configurable options | SATISFIED | 720x500 sidebar-nav settings window with five sections: General (hotkey + recording mode), Profiles (profiles + corrections + ALL CAPS), Model, Microphone, Appearance |
| SET-03 | 06-02 | User can select whisper model | SATISFIED | list_models and set_model Tauri commands, ModelSelector UI with three models, download status, recommended badge, runtime reload via spawn_blocking |
| SET-04 | 06-02 | User can select microphone | SATISFIED | list_input_devices and set_microphone Tauri commands, MicrophoneSection dropdown, AudioCaptureMutex allows runtime stream replacement |

All 9 required requirement IDs (VOC-01 through VOC-06, SET-01, SET-03, SET-04) are covered. No orphaned requirements — REQUIREMENTS.md traceability table shows all 9 mapped to Phase 6.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| src/components/DictionaryEditor.tsx | 91, 101 | `placeholder="why section"` / `placeholder="W-section"` | Info | HTML input placeholder attributes — not code stubs, informational |
| src/components/ProfileSwitcher.tsx | 6 | `active: boolean` in ProfileInfo interface | Warning | Backend returns `is_active` (serialized as `isActive` by Tauri). Frontend TS declares `active`. Field is never read (selection uses `activeId` prop) so no runtime failure, but type is incorrect |

### Wiring Issue — Corrections Save

**Issue: `save_corrections` parameter naming mismatch**

- Frontend (ProfilesSection.tsx line 51): `invoke('save_corrections', { corrections: updated })`
- Backend (lib.rs line 496): `fn save_corrections(app: tauri::AppHandle, corrections_map: HashMap<String, String>)`

In Tauri 2, command arguments are matched by camelCase-to-snake_case name conversion. The JS key `corrections` maps to Rust parameter named `corrections`, not `corrections_map`. This means `corrections_map` receives an empty HashMap when `save_corrections` is called.

Functional impact:
- `guard.corrections.extend(corrections_map.clone())` — extends in-memory corrections with empty map (no change)
- `json[&key] = serde_json::to_value(&corrections_map).unwrap()` — persists empty object `{}` to disk for user corrections
- In-session: corrections appear to work because `get_corrections` returns in-memory state (previously loaded from profile defaults + persisted user corrections)
- After restart: `read_saved_corrections` returns empty HashMap, user-added corrections are lost

This may have passed human verification because: the human added "test" → "TEST RESULT", dictated "test", saw "TEST RESULT" (in-session, working from memory). The persistence test in step 10 focused on ALL CAPS, selected profile, model, mic, theme, and hotkey — but did not specifically re-test a user-added custom correction entry after restart.

### Human Verification Required

#### 1. Corrections Dictionary Persistence

**Test:** Add a correction entry in the Profiles section (From: "mpa" / To: "MPa" on the General profile), close the settings window, quit the app from tray, reopen the app, open Settings > Profiles section.
**Expected:** The "mpa" → "MPa" correction entry must still appear in the dictionary table.
**Why human:** Requires app restart and reopen to verify disk persistence. The parameter mismatch may cause silent failure where the entry appears correct in-session (from memory) but disappears after restart.

#### 2. Whisper initial_prompt bias effectiveness

**Test:** Switch to General profile, dictate "why section W8x31 ACI 318". Note the transcription. Switch to Structural Engineering profile, dictate the same phrase again.
**Expected:** Structural Engineering profile should produce more accurate transcription of engineering terms (corrections will also apply in SE profile — "why section" becomes "W-section").
**Why human:** Requires live audio input and whisper inference — cannot verify programmatically.

#### 3. Model selector download status accuracy

**Test:** Open Settings > Model section. Check each model card's download indicator against actual files in %APPDATA%\VoiceType\models\.
**Expected:** Cards show "Not downloaded" for missing model files and are clickable only for downloaded models.
**Why human:** Requires running app with list_models command — file existence is runtime state.

### Gaps Summary

One gap blocks full goal achievement:

**Corrections persistence failure** (`save_corrections` parameter mismatch): User-edited corrections dictionary entries are not saved to disk. The `save_corrections` Tauri command receives an empty map because the frontend sends `{ corrections: updated }` but the Rust parameter is named `corrections_map`. Tauri 2's camelCase-to-snake_case conversion maps `corrections` → `corrections`, not `corrections_map`. The fix is one line: rename the Rust parameter from `corrections_map` to `corrections` in the `save_corrections` function signature and all usages within that function.

This does not affect the default corrections (Structural Engineering profile's 13 built-in corrections) which are loaded from the profile definition, not from settings.json user storage. Only user-added or user-modified corrections are affected.

---

_Verified: 2026-02-28_
_Verifier: Claude (gsd-verifier)_
