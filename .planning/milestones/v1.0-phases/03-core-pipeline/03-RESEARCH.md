# Phase 3: Core Pipeline - Research

**Researched:** 2026-02-27
**Domain:** Text injection (clipboard + enigo), pipeline state machine, tray icon state, Tauri command wiring
**Confidence:** HIGH (core stack verified via official docs + crates.io; timing patterns verified via keyless reference + community sources)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Text formatting**
- Trust whisper's sentence structure output (capitalization, punctuation) — no additional sentence formatting on top
- Trim leading whitespace from whisper output before injection
- Append trailing space after injected text to bridge consecutive dictations naturally
- Hallucination filtering: Claude's discretion on whether to add lightweight known-pattern stripping (repeated phrases, "Thank you for watching" artifacts) or defer entirely to Phase 5 VAD gating

**Multi-dictation behavior**
- Each dictation is an independent insert at current cursor position — no accumulation buffer
- Block new recording while pipeline is processing (hotkey ignored during whisper inference + injection) — prevents race conditions and clipboard conflicts
- Always inject into whatever app has focus when hotkey is released — no target app tracking
- No cooldown between dictations — ready for next as soon as injection + clipboard restore completes

**Error & empty handling**
- Empty/whitespace-only whisper results: silent discard, no clipboard touch, return to idle
- Clipboard restore failure: log the failure, move on — text was already injected, clipboard loss is a known edge case
- Paste failure: best-effort, no retry — user will notice and re-dictate
- Whisper inference errors: log and return to idle silently

**Pre-overlay feedback**
- System tray icon changes to indicate state — three distinct states:
  - **Idle**: normal app icon
  - **Recording**: active/red icon while hotkey is held
  - **Processing**: different icon/spinner while whisper runs after release
- No audio cues — silent operation, tray icon is the only feedback
- These three states map directly to the Phase 4 overlay states later

### Claude's Discretion
- Whether to add lightweight hallucination filtering now or defer to Phase 5 VAD
- Tray tooltip showing last transcription result (useful for debugging vs unnecessary clutter)
- Text command expansion (e.g., "new line" → newline) — decide if any quick wins are worth adding without overcomplicating

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| CORE-05 | Transcribed text is injected at the active cursor position via clipboard paste (Ctrl+V) | arboard for clipboard, enigo for Ctrl+V simulation; standard pattern verified in keyless reference project |
| CORE-06 | App saves clipboard contents before injection and restores them after (with timing delays to avoid race conditions) | arboard get_text/set_text; 50-100ms pre-paste sleep + 100-150ms pre-restore sleep documented as Windows requirement |
| REC-01 | User can hold the hotkey to record and release to transcribe (hold-to-talk mode) | tauri-plugin-global-shortcut ShortcutState::Pressed/Released; existing hotkey handler in lib.rs currently only handles Pressed — needs Released wiring |
</phase_requirements>

---

## Summary

Phase 3 wires three things that are already individually built (Phase 1 hotkey + Phase 2 audio capture + Phase 2 whisper inference) into a working end-to-end pipeline: hotkey hold starts recording, hotkey release stops recording and fires whisper inference, transcription result is injected at cursor via clipboard-paste, original clipboard is restored. The tray icon reflects the three states (idle/recording/processing) to give the user pre-overlay feedback.

The text injection stack is settled: **arboard** for clipboard save/restore and **enigo** for Ctrl+V simulation. This is the exact stack used by the keyless reference project (the closest analog to VoiceType in the Rust ecosystem). The clipboard timing requirements are real and documented: a 50-100ms sleep before Ctrl+V gives Windows time to propagate the clipboard write, and a 100-150ms sleep before restore gives the target app time to consume the paste. Skipping delays causes intermittent failures in Chrome and VS Code.

The biggest code change in Phase 3 is the hotkey handler. The existing handler in `lib.rs` only handles `ShortcutState::Pressed`. It must be extended to handle `ShortcutState::Released` and coordinate the full pipeline. Pipeline concurrency control — blocking new recordings during inference+injection — is implemented with an `AtomicBool` (or `AtomicU8` for 3-state) in managed state, checked via `compare_exchange` to prevent races. The tray icon is updated at each state transition using `app.tray_by_id("tray").set_icon()`, which is Tauri 2's runtime icon-change mechanism.

**Primary recommendation:** Use arboard + enigo (same as keyless). Wire hold/release in the existing global-shortcut handler. Use `AtomicU8` with named constants for pipeline state. Store three icon byte slices as `include_bytes!` constants. Run the injection sequence in `tokio::spawn_blocking` since enigo and arboard are synchronous.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| arboard | 3.6.1 | Clipboard get_text / set_text | 1Password-maintained, 25k+ dependents, cleanest API for save/restore, same as keyless reference project; avoids raw Win32 GlobalAlloc complexity |
| enigo | 0.6.1 | Ctrl+V keyboard simulation | Cross-platform input simulation, Key::Control + Key::Unicode('v') pattern is the standard; same as keyless reference project |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tauri-plugin-global-shortcut | 2.x (already in project) | ShortcutState::Released for hold-to-talk | Already integrated — extend existing handler to handle Released |
| std::sync::atomic::AtomicU8 | std | 3-state pipeline lock (idle/recording/processing) | No additional crate needed; `compare_exchange` prevents race between hotkey presses |
| tokio::task::spawn_blocking | already in Tauri runtime | Run arboard + enigo (blocking) off async thread | Already used for whisper inference in transcribe commands; same pattern |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| arboard | clipboard-win 5.4.1 | clipboard-win is Windows-only; arboard is cross-platform and has simpler API (get_text/set_text vs Getter/Setter traits); both work, arboard preferred |
| arboard | tauri-plugin-clipboard-manager | Plugin requires capability config and JS bridge; raw arboard from Rust is simpler for backend-only clipboard ops |
| enigo | raw Win32 SendInput | SendInput is more reliable in some edge cases but requires unsafe + windows-rs feature flags; enigo abstracts this and is proven on Windows |
| AtomicU8 | Mutex\<PipelineState\> | Mutex adds lock overhead; AtomicU8 with compare_exchange is sufficient for this 3-state machine since transitions are all from the Tauri command thread |

**Installation:**
```toml
# In src-tauri/Cargo.toml
arboard = "3"
enigo = "0.6"
```

---

## Architecture Patterns

### Recommended Module Structure

```
src-tauri/src/
├── audio.rs          # existing — no changes
├── transcribe.rs     # existing — no changes
├── tray.rs           # extend: add set_tray_state() helper
├── inject.rs         # NEW — clipboard save/restore + Ctrl+V paste
├── pipeline.rs       # NEW — PipelineState AtomicU8 + orchestration logic
└── lib.rs            # extend hotkey handler for Released; wire pipeline
```

### Pattern 1: Pipeline State Machine with AtomicU8

**What:** Three-state machine (Idle=0, Recording=1, Processing=2) using AtomicU8 stored in Tauri managed state. `compare_exchange` from Idle→Recording on Pressed, Recording→Processing on Released. Reset to Idle after injection completes or on any error.

**When to use:** All pipeline state transitions. The atomic compare_exchange provides lock-free concurrency control — if hotkey is pressed while Processing, compare_exchange fails and the event is silently ignored (this implements "block new recording while pipeline is processing").

**Example:**
```rust
// pipeline.rs
use std::sync::atomic::{AtomicU8, Ordering};

pub const IDLE: u8 = 0;
pub const RECORDING: u8 = 1;
pub const PROCESSING: u8 = 2;

pub struct PipelineState(pub AtomicU8);

impl PipelineState {
    pub fn new() -> Self {
        PipelineState(AtomicU8::new(IDLE))
    }

    /// Try to transition from `from` to `to`. Returns true if successful.
    pub fn transition(&self, from: u8, to: u8) -> bool {
        self.0.compare_exchange(from, to, Ordering::SeqCst, Ordering::Relaxed).is_ok()
    }

    pub fn set(&self, val: u8) {
        self.0.store(val, Ordering::SeqCst);
    }

    pub fn get(&self) -> u8 {
        self.0.load(Ordering::SeqCst)
    }
}
```

### Pattern 2: Hold-to-Talk Hotkey Handler

**What:** Extend the existing global-shortcut handler in `lib.rs::run()` to handle both `ShortcutState::Pressed` and `ShortcutState::Released`. On Pressed: transition IDLE→RECORDING, start audio, update tray. On Released: transition RECORDING→PROCESSING, stop audio, spawn pipeline task.

**Critical finding:** The existing handler in `lib.rs` currently only handles `ShortcutState::Pressed` and emits `hotkey-triggered` to the frontend. This must be replaced with a backend-driven pipeline for Phase 3.

**Example:**
```rust
// In lib.rs setup(), replace the existing shortcut handler:
.with_handler(|app, _shortcut, event| {
    use tauri_plugin_global_shortcut::ShortcutState;
    let pipeline = app.state::<PipelineState>();

    match event.state {
        ShortcutState::Pressed => {
            // Only start if idle — ignore if recording or processing
            if pipeline.transition(IDLE, RECORDING) {
                let audio = app.state::<AudioCapture>();
                audio.clear_buffer();
                audio.recording.store(true, Ordering::Relaxed);
                set_tray_state(app, TrayState::Recording);
                log::info!("Pipeline: IDLE -> RECORDING");
            }
        }
        ShortcutState::Released => {
            // Only fire if we were recording (not if hotkey was blocked)
            if pipeline.transition(RECORDING, PROCESSING) {
                set_tray_state(app, TrayState::Processing);
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    run_pipeline(app_handle).await;
                });
            }
        }
    }
})
```

### Pattern 3: Clipboard Save/Restore + Paste Injection

**What:** save clipboard → set clipboard to transcription → sleep 50-100ms → Ctrl+V → sleep 100-150ms → restore clipboard → return.

**Timing is required and documented:** Windows clipboard writes are asynchronous from the perspective of the target app. Without the pre-paste sleep, some apps (Chrome, VS Code) paste the wrong content. Without the pre-restore sleep, the restore overwrites the clipboard before the paste has been consumed.

**Example:**
```rust
// inject.rs
use arboard::Clipboard;
use enigo::{Direction::{Click, Press, Release}, Enigo, Key, Keyboard, Settings};
use std::thread;
use std::time::Duration;

pub fn inject_text(text: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;

    // Save existing clipboard content (may be empty or non-text)
    let saved = clipboard.get_text().ok(); // Ok(None) if non-text or empty

    // Write transcription to clipboard
    clipboard.set_text(text).map_err(|e| e.to_string())?;

    // Allow clipboard write to propagate before paste
    thread::sleep(Duration::from_millis(75)); // 50-100ms range; 75ms is the midpoint

    // Simulate Ctrl+V
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    enigo.key(Key::Control, Press).map_err(|e| e.to_string())?;
    enigo.key(Key::Unicode('v'), Click).map_err(|e| e.to_string())?;
    enigo.key(Key::Control, Release).map_err(|e| e.to_string())?;

    // Allow target app to consume the paste before restore
    thread::sleep(Duration::from_millis(120)); // 100-150ms range

    // Restore original clipboard content
    match saved {
        Some(original) => {
            if let Err(e) = clipboard.set_text(&original) {
                log::warn!("Clipboard restore failed: {} — clipboard contents lost", e);
            }
        }
        None => {
            // Original was empty or non-text — clear clipboard
            // Note: arboard has no clear() in all versions; set empty string as fallback
            let _ = clipboard.set_text("");
        }
    }

    Ok(())
}
```

### Pattern 4: Tray Icon State Changes

**What:** Use `app.tray_by_id("tray")` to get the runtime TrayIcon handle, then call `set_icon()` with pre-embedded icon bytes. The tray must be built with `.with_id("tray")` in `build_tray()`.

**Key finding:** `tray_by_id()` is the Tauri 2 runtime mechanism — do NOT store TrayIcon in managed state. It is reference-counted by Tauri internally and accessible by ID from any AppHandle.

**Example:**
```rust
// tray.rs — add:
pub enum TrayState { Idle, Recording, Processing }

static ICON_IDLE: &[u8] = include_bytes!("../icons/tray-idle.ico");
static ICON_RECORDING: &[u8] = include_bytes!("../icons/tray-recording.ico");
static ICON_PROCESSING: &[u8] = include_bytes!("../icons/tray-processing.ico");

pub fn set_tray_state(app: &tauri::AppHandle, state: TrayState) {
    let icon_bytes = match state {
        TrayState::Idle => ICON_IDLE,
        TrayState::Recording => ICON_RECORDING,
        TrayState::Processing => ICON_PROCESSING,
    };
    if let Some(tray) = app.tray_by_id("tray") {
        if let Ok(image) = tauri::image::Image::from_bytes(icon_bytes) {
            let _ = tray.set_icon(Some(image));
        }
    }
}

// In build_tray(), change TrayIconBuilder::new() to TrayIconBuilder::with_id("tray")
```

### Pattern 5: Pipeline Orchestration (run_pipeline)

**What:** Async function called from the Released handler. Gets audio buffer, runs whisper inference in spawn_blocking, applies text formatting, injects text, resets state.

**Example:**
```rust
// pipeline.rs
async fn run_pipeline(app: tauri::AppHandle) {
    // 1. Get audio samples
    let audio = app.state::<AudioCapture>();
    let samples = audio.get_buffer();

    if samples.len() < 1600 { // < 100ms of audio at 16kHz — ignore
        log::info!("Pipeline: audio too short, discarding");
        app.state::<PipelineState>().set(IDLE);
        set_tray_state(&app, TrayState::Idle);
        return;
    }

    // 2. Run whisper inference (blocking)
    #[cfg(feature = "whisper")]
    let result = {
        let whisper_state = app.state::<WhisperState>();
        let ctx = match &whisper_state.0 {
            Some(ctx) => ctx.clone(),
            None => {
                log::error!("Pipeline: whisper not loaded");
                app.state::<PipelineState>().set(IDLE);
                set_tray_state(&app, TrayState::Idle);
                return;
            }
        };
        tokio::task::spawn_blocking(move || transcribe::transcribe_audio(&ctx, &samples))
            .await
    };

    // 3. Handle result
    let text = match result {
        Ok(Ok(t)) => t,
        Ok(Err(e)) => {
            log::error!("Pipeline: whisper error: {}", e);
            app.state::<PipelineState>().set(IDLE);
            set_tray_state(&app, TrayState::Idle);
            return;
        }
        Err(e) => {
            log::error!("Pipeline: spawn_blocking panicked: {}", e);
            app.state::<PipelineState>().set(IDLE);
            set_tray_state(&app, TrayState::Idle);
            return;
        }
    };

    // 4. Format: trim + trailing space; discard empty
    let trimmed = text.trim_start().to_string();
    if trimmed.is_empty() || trimmed.chars().all(|c| c.is_whitespace()) {
        log::info!("Pipeline: empty transcription, discarding");
        app.state::<PipelineState>().set(IDLE);
        set_tray_state(&app, TrayState::Idle);
        return;
    }
    let to_inject = format!("{} ", trimmed); // trailing space for consecutive dictation

    // 5. Inject (blocking — arboard + enigo are sync)
    let inject_result = tokio::task::spawn_blocking(move || inject_text(&to_inject)).await;
    match inject_result {
        Ok(Ok(())) => log::info!("Pipeline: injection complete"),
        Ok(Err(e)) => log::error!("Pipeline: injection failed: {}", e),
        Err(e) => log::error!("Pipeline: injection panicked: {}", e),
    }

    // 6. Reset state
    app.state::<PipelineState>().set(IDLE);
    set_tray_state(&app, TrayState::Idle);
}
```

### Anti-Patterns to Avoid

- **Calling `clipboard.set_text()` then immediately sending Ctrl+V:** The clipboard write is not synchronous from the target app's perspective on Windows. The 50-100ms sleep is mandatory.
- **Running arboard or enigo in an async context without spawn_blocking:** Both are synchronous, blocking APIs. Calling them directly in an async function stalls the Tauri runtime.
- **Using `app.emit("hotkey-triggered", ())` for pipeline coordination:** The existing frontend-based approach cannot coordinate the backend pipeline state. Phase 3 must be backend-driven.
- **Storing TrayIcon in managed state:** Use `tray_by_id("tray")` instead — Tauri manages the lifecycle.
- **Using `lock()` instead of `try_lock()` in audio callbacks:** Already documented in audio.rs comments — maintain this pattern.
- **Sharing one `Enigo` instance across calls:** Enigo::new() is cheap; create a fresh instance per injection to avoid state accumulation.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Clipboard text save/restore | Manual Win32 GlobalAlloc + GlobalLock + CF_UNICODETEXT | arboard 3.6.1 | GlobalAlloc clipboard pattern requires ~60 lines of unsafe code, GMEM_MOVEABLE flags, manual null-termination in UTF-16; arboard handles all of this |
| Keyboard Ctrl+V simulation | Raw Win32 SendInput with INPUT structs | enigo 0.6.1 | SendInput requires unsafe code, INPUT struct packing, and VK_CONTROL + VK_V virtual key codes; enigo abstracts this cleanly |
| 3-state pipeline lock | Mutex\<enum\> | AtomicU8 with named constants | Mutex adds contention overhead; AtomicU8 compare_exchange is lock-free and sufficient for this simple state machine |

**Key insight:** The clipboard-paste injection pattern is deceptively complex at the Win32 level. arboard + enigo is the established Rust solution with production validation in keyless.

---

## Common Pitfalls

### Pitfall 1: Clipboard Timing Race (Most Critical)
**What goes wrong:** Target app pastes stale clipboard content instead of transcription, OR transcription is injected but original clipboard is lost.
**Why it happens:** Windows clipboard writes are asynchronous. The target app's message loop must process `WM_CLIPBOARDUPDATE` before a Ctrl+V paste reads from the clipboard. Similarly, the paste processing is asynchronous — restoring the clipboard too early overwrites before the paste is consumed.
**How to avoid:** Sleep 50-100ms after `set_text()` before sending Ctrl+V. Sleep 100-150ms after Ctrl+V before calling `set_text(original)` to restore.
**Warning signs:** Works in Notepad (fast message loop) but fails intermittently in Chrome or VS Code.

### Pitfall 2: hotkey-triggered Event Still Fired
**What goes wrong:** Frontend receives `hotkey-triggered` events from the old `Pressed`-only handler after Phase 3 refactor, causing UI confusion.
**Why it happens:** The existing lib.rs handler emits to the frontend. Phase 3 replaces this with backend-driven pipeline.
**How to avoid:** Remove or conditionally disable the `app.emit("hotkey-triggered", ())` call in setup() when the pipeline handler takes over. The frontend tray state will be driven by icon changes, not events.

### Pitfall 3: Pipeline Does Not Reset to IDLE on Error
**What goes wrong:** App appears frozen — hotkey does nothing because pipeline is stuck in PROCESSING state.
**Why it happens:** An early return in `run_pipeline` forgets to reset `PipelineState` and update tray.
**How to avoid:** Every exit path (early return, error branch, success) must call `set(IDLE)` and `set_tray_state(Idle)`. Use a defer pattern or explicit cleanup in every branch.
**Warning signs:** Tray icon stays in "processing" state indefinitely.

### Pitfall 4: Enigo / arboard in Async Context Without spawn_blocking
**What goes wrong:** Tauri async runtime stalls during clipboard or keyboard operations.
**Why it happens:** arboard and enigo are blocking APIs. Calling them directly in an `async fn` blocks the executor thread.
**How to avoid:** Always wrap arboard + enigo calls in `tokio::task::spawn_blocking`. This is already established as the pattern for whisper inference — apply the same approach.

### Pitfall 5: Tray Icon Not Found at Runtime
**What goes wrong:** `app.tray_by_id("tray")` returns None and icon never changes.
**Why it happens:** `TrayIconBuilder::new()` (no ID) was used instead of `TrayIconBuilder::with_id("tray")`.
**How to avoid:** Phase 3 Plan 1 must change `build_tray()` to use `TrayIconBuilder::with_id("tray")`.
**Warning signs:** No compile error; silent failure at runtime.

### Pitfall 6: Icon Files Not Present at Compile Time
**What goes wrong:** `include_bytes!("../icons/tray-recording.ico")` fails to compile because the file doesn't exist.
**Why it happens:** New icon assets must be created before wiring up `set_tray_state()`.
**How to avoid:** Phase 3 Plan 1 (text injection) should create placeholder icon files early. Derive them from the existing `icons/icon.ico` with color tinting — a simple approach for now.

### Pitfall 7: arboard Non-Text Clipboard Restore Failure
**What goes wrong:** User had an image or rich-text on clipboard before dictation; after restore, clipboard is empty (arboard returns Err on get_text for non-Unicode formats).
**Why it happens:** `get_text()` returns Err if clipboard contains non-text content. The code must handle this gracefully.
**How to avoid:** Use `.ok()` on `get_text()` to convert to `Option<String>`. If None, skip restore (or clear clipboard). Document this as a known limitation per user decisions (clipboard restore failure: log, move on).

---

## Code Examples

Verified patterns from official sources and the keyless reference project:

### arboard Save/Restore
```rust
// Source: docs.rs/arboard/3.6.1/arboard/struct.Clipboard.html
let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
let saved: Option<String> = clipboard.get_text().ok(); // None if non-text content
clipboard.set_text("text to paste").map_err(|e| e.to_string())?;
// ... sleep, paste, sleep ...
if let Some(original) = saved {
    let _ = clipboard.set_text(&original); // best-effort restore per user decisions
}
```

### enigo Ctrl+V
```rust
// Source: github.com/enigo-rs/enigo/blob/main/examples/keyboard.rs
use enigo::{Direction::{Click, Press, Release}, Enigo, Key, Keyboard, Settings};
let mut enigo = Enigo::new(&Settings::default()).unwrap();
enigo.key(Key::Control, Press).unwrap();
enigo.key(Key::Unicode('v'), Click).unwrap();
enigo.key(Key::Control, Release).unwrap();
```

### Tauri 2 Runtime Tray Icon Change
```rust
// Source: dev.to/rain9/tauri5-tray-icon-implementation-and-event-handling-5d1e
// Requires: TrayIconBuilder::with_id("tray") in build_tray()
if let Some(tray) = app.tray_by_id("tray") {
    if let Ok(image) = tauri::image::Image::from_bytes(icon_bytes) {
        let _ = tray.set_icon(Some(image));
    }
}
```

### ShortcutState::Released Handler
```rust
// Source: v2.tauri.app/plugin/global-shortcut/
match event.state {
    ShortcutState::Pressed => { /* start recording */ }
    ShortcutState::Released => { /* stop recording, fire pipeline */ }
}
```

### AtomicU8 State Transition (compare_exchange)
```rust
// Source: doc.rust-lang.org/std/sync/atomic/struct.AtomicU8.html
// Transition IDLE -> RECORDING, returns true only if was IDLE
let ok = state.compare_exchange(IDLE, RECORDING, Ordering::SeqCst, Ordering::Relaxed).is_ok();
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Emit hotkey event to frontend, frontend calls start/stop commands | Backend-driven pipeline: Released handler owns the full loop | Phase 3 | Eliminates JS round-trip latency and race conditions between frontend state and backend audio |
| tauri-plugin-clipboard-manager (requires capability config) | Direct arboard in Rust backend | N/A — never used in this project | Simpler, no JS bridge, no capability TOML changes needed |
| TrayIconBuilder::new() (no ID) | TrayIconBuilder::with_id("tray") | Phase 3 plan 1 | Required to call tray_by_id() for runtime icon changes |

**Deprecated/outdated:**
- `menu_on_left_click(false)` → replaced by `show_menu_on_left_click(false)` in Tauri 2.10.x (already handled in Phase 1, documented in STATE.md)

---

## Open Questions

1. **Tray icon artwork for recording/processing states**
   - What we know: Three ICO files are needed (idle, recording, processing); idle = existing icon.ico
   - What's unclear: Whether to create color-tinted variants programmatically or use hand-crafted assets; whether 16x16 or 32x32 ICO is appropriate for Windows 11 system tray
   - Recommendation: Start with simple single-color ICO files (red for recording, orange/yellow for processing) derived from icon.ico by manual edit in a paint tool. 16x16 is sufficient for system tray.

2. **Hallucination filter: now or Phase 5?**
   - What we know: Whisper commonly emits "Thank you for watching.", "Thank you.", and repetition artifacts for short silence segments
   - What's unclear: Whether these will realistically appear during hold-to-talk dictation (user holds key while speaking, so silence segments are minimal)
   - Recommendation: Defer to Phase 5. Hold-to-talk inherently limits silence; hallucination artifacts are primarily a Phase 5 VAD problem.

3. **Tray tooltip: show last transcription or not?**
   - What we know: `tray.set_tooltip(Some("Last: ..."))` is straightforward with arboard
   - What's unclear: Whether it adds enough debug value vs noise
   - Recommendation: Add it — useful during development for confirming transcription results without a log viewer. Remove or gate behind a debug build if it feels cluttered.

---

## Sources

### Primary (HIGH confidence)
- docs.rs/arboard/3.6.1 — get_text, set_text, Windows threading behavior, sleep recommendation
- docs.rs/enigo/0.6.1 — Enigo::new, Key::Control, Key::Unicode, Direction Press/Click/Release
- v2.tauri.app/plugin/global-shortcut/ — ShortcutState::Pressed and ShortcutState::Released handler pattern
- docs.rs/tauri/latest/tauri/tray/struct.TrayIcon.html — set_icon method signature, Image::from_bytes
- doc.rust-lang.org/std/sync/atomic — AtomicU8, compare_exchange semantics

### Secondary (MEDIUM confidence)
- github.com/hate/keyless — reference project using arboard + enigo for dictation paste; confirms stack is production-viable; architecture patterns observed
- dev.to/rain9/tauri5-tray-icon-implementation-and-event-handling — tray_by_id() runtime icon-change pattern; matches tauri docs

### Tertiary (LOW confidence)
- AutoHotkey community discussions — timing delays (50ms pre-paste, 100-150ms pre-restore) observed in AHK clipboard operations; consistent with arboard sleep recommendation; applies to Windows clipboard behavior generally

---

## Metadata

**Confidence breakdown:**
- Standard stack (arboard + enigo): HIGH — official docs verified, confirmed in keyless reference project
- Architecture (pipeline state machine, tray icon): HIGH — Tauri 2 docs + working discussion examples
- Timing delays (50-100ms pre-paste, 100-150ms pre-restore): MEDIUM — arboard doc mentions sleep helps; AHK community confirms Windows clipboard timing; specific ms values from community practice, not official spec
- Pitfalls: HIGH for items derived from existing STATE.md decisions; MEDIUM for new timing pitfalls

**Research date:** 2026-02-27
**Valid until:** 2026-03-27 (stable libraries — arboard, enigo, Tauri 2 API all in stable release)
