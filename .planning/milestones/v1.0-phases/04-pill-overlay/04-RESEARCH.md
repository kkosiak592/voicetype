# Phase 4: Pill Overlay - Research

**Researched:** 2026-02-28
**Domain:** Tauri 2 multi-window, Win32 focus management, React audio visualizer, CSS animation
**Confidence:** MEDIUM-HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Pill appearance:**
- Compact size (~120x40px)
- Always dark solid background — does not follow light/dark theme setting
- Fully rounded pill/capsule shape (semicircular ends)
- Heavily frosted opacity (~70-80%) — mostly opaque, slight see-through

**Visualizer design:**
- Frequency bars style — classic vertical equalizer bars
- ~15 bars to fill the compact pill width
- Bar color: Claude's discretion (pick what complements dark pill)
- Animation: follow best practice/standard for audio visualizers (smooth interpolation typical)

**Screen position & behavior:**
- Default position: bottom center of screen
- Draggable: user can drag the pill anywhere on screen
- Position persistence: remember last drag position across sessions (tauri-plugin-store)
- Show/hide transition: fade in/out (smooth opacity transition)
- Hidden when idle — pill only visible during recording and processing states

**State display:**
- **Recording**: frequency bars animate with mic input + small red recording dot indicator
- **Processing**: wavy/animated pill border effect — modern, fluid border animation while whisper transcribes (bars go static or disappear)
- **Completion**: brief success flash (~300ms color flash or checkmark) then fade out
- **Error**: brief error flash (red/orange ~500ms) for no-speech-detected or whisper failures, then fade out

### Claude's Discretion
- Exact bar color choice for frequency visualizer
- Animation easing/timing curves
- Specific implementation of the wavy border processing effect (CSS animation, canvas, SVG — whichever achieves the modern fluid look)
- Opacity level fine-tuning within the 70-80% range
- Exact flash duration and color for success/error states
- Spacing and padding inside the pill

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| UI-01 | A floating pill-shaped overlay appears on screen during recording (always-on-top, transparent, frameless) | Tauri 2 WebviewWindowBuilder with `transparent(true)`, `decorations(false)`, `always_on_top(true)`, `visible(false)` at start; show/hide via `app.emit_to` events |
| UI-02 | The pill overlay does not steal focus from the active application (Win32 WS_EX_NOACTIVATE) | `focusable(false)` on WebviewWindowBuilder is the Tauri-native approach; Win32 `WS_EX_NOACTIVATE` via `raw_window_handle` + `windows-rs` crate as fallback if `focusable(false)` proves insufficient |
| UI-03 | The pill displays an audio visualizer with frequency bars showing mic input levels | RMS computed from `audio.buffer` (existing Arc<Mutex<Vec<f32>>>), emitted via `app.emit_to("pill", ...)` at ~30fps, rendered in React with CSS/Canvas bars |
| UI-04 | The pill shows recording state (idle/recording/processing) | PipelineState AtomicU8 transitions already exist; add `app.emit_to("pill", "pill-state", ...)` at each IDLE/RECORDING/PROCESSING transition point in lib.rs hotkey handler |
</phase_requirements>

---

## Summary

Phase 4 builds a second Tauri window — the pill overlay — that is frameless, transparent, always-on-top, and never steals focus. The two hardest problems are (1) guaranteed no-focus-steal on Windows and (2) streaming audio RMS levels to the frontend fast enough for a responsive visualizer.

For focus prevention, Tauri 2's `WebviewWindowBuilder::focusable(false)` is the correct first-line approach — it was added specifically to address the broken `focus: false` config behavior (issue #11566, fixed late 2024). The STATE.md note about "WS_EX_NOACTIVATE required, config alone broken" referred to the old config-file approach, which is now fixed via the Rust builder API. However, dragging a no-focus window has a known complication: `data-tauri-drag-region` requires the window to be focused to initiate drag on Windows (#11605). The workaround is to use `window.startDragging()` programmatically from a `mousedown` handler, paired with position save/restore via `tauri-plugin-store`.

For the visualizer, the correct approach is: Rust audio callback computes RMS from the most recent N samples of `audio.buffer`, spawns a background loop that emits a `pill-level` event via `app.emit_to("pill", ...)` at ~30fps during recording only. The React component listens with `getCurrentWebviewWindow().listen("pill-level", ...)` and updates bar heights via React state. No Web Audio API is needed — the audio data is already in Rust.

**Primary recommendation:** Use `WebviewWindowBuilder` in Rust `setup()` with `focusable(false)` + `always_on_top(true)` + `transparent(true)` + `decorations(false)` + `skip_taskbar(true)` + `visible(false)`. Emit state and level events from existing pipeline transition points. Use CSS `@property` + `conic-gradient` for the animated processing border.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tauri (existing) | 2.x | Second window creation, event emission | Already in project — `WebviewWindowBuilder` API |
| tauri-plugin-store (existing) | 2.x | Pill position persistence | Already used for settings; exact same API |
| React + Tailwind CSS (existing) | 18.x / 4.x | Pill.tsx component | Established project frontend stack |
| @tauri-apps/api (existing) | 2.x | `listen()`, `getCurrentWebviewWindow()`, `startDragging()` | Already in package.json |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| windows-rs crate | 0.58+ | Win32 `SetWindowLongPtrW` for `WS_EX_NOACTIVATE` | Only if `focusable(false)` proves insufficient after testing |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `focusable(false)` builder method | Win32 `WS_EX_NOACTIVATE` via windows-rs | Win32 is more guaranteed but requires unsafe Rust + extra crate; try builder method first |
| `app.emit_to` events for audio levels | Tauri Channel | Channel is better for high-throughput one-shot streams; emit_to is fine for 30fps level updates |
| Rust RMS calculation | Web Audio API AnalyserNode | Web Audio requires the audio to be in the browser context — our audio is in Rust, so Rust RMS is the only option |
| CSS animated border | Canvas or SVG border animation | CSS `@property` + `conic-gradient` is cleanest for a pill border; WebView2 supports all modern CSS |

**Installation (no new packages required):**

The pill requires no new npm or Cargo dependencies. All needed APIs are already present. The only possible addition is `windows` crate if fallback Win32 manipulation is needed:

```toml
# Cargo.toml — only add if focusable(false) is insufficient:
[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.58", features = ["Win32_UI_WindowsAndMessaging"] }
```

---

## Architecture Patterns

### Recommended Project Structure

```
src/
├── App.tsx              # settings window (unchanged)
├── Pill.tsx             # NEW: pill overlay entry component
├── components/
│   ├── FrequencyBars.tsx   # NEW: ~15 animated vertical bars
│   └── PillStateDisplay.tsx  # NEW: recording dot, state switcher
├── lib/
│   └── store.ts         # existing — reuse for pill position
└── main.tsx             # settings window entry (unchanged)

src-tauri/src/
├── lib.rs               # add pill window creation + event emission
├── pipeline.rs          # add emit_to pill events at state transitions
├── audio.rs             # add RMS streaming loop (background task)
└── pill.rs              # NEW: pill-specific event types + RMS helper

src-tauri/
├── tauri.conf.json      # add pill window definition
└── capabilities/
    ├── default.json     # add "pill" to windows list
    └── desktop.json     # add "pill" to windows list
```

### Pattern 1: Second Window Definition in tauri.conf.json

**What:** Define the pill window statically in config so Tauri knows about it at startup. Window starts hidden (`visible: false`).
**When to use:** Any time a permanent secondary window is needed; this avoids runtime window creation which can deadlock in sync contexts.

```json
// tauri.conf.json — add to app.windows array
{
  "label": "pill",
  "url": "pill.html",
  "width": 120,
  "height": 40,
  "transparent": true,
  "decorations": false,
  "alwaysOnTop": true,
  "resizable": false,
  "visible": false,
  "skipTaskbar": true
}
```

**Critical note:** tauri.conf.json supports `alwaysOnTop`, `decorations`, `transparent`, `visible`, `skipTaskbar` — but `focusable` must be set in Rust setup() via `get_webview_window("pill").set_focusable(false)` after the window is created, or the window must be re-created programmatically. The `focusable` property is NOT available as a tauri.conf.json key — only via the Rust builder or post-creation API.

### Pattern 2: Post-Creation Focusable Configuration in setup()

**What:** After setup creates the pill window from config, immediately configure focus-prevention. This is the recommended approach since `focusable` is builder-only.
**When to use:** Any overlay that must never steal focus on Windows.

```rust
// src-tauri/src/lib.rs — in setup() after build_tray()
// Source: https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindowBuilder.html

#[cfg(target_os = "windows")]
if let Some(pill) = app.get_webview_window("pill") {
    // focusable(false) sets WS_EX_NOACTIVATE equivalent behavior
    // This must be called after window creation, not in config
    let _ = pill.set_ignore_cursor_events(false); // pill IS clickable for drag
    // Note: set_focusable API — verify against current tauri version
}
```

**Alternative: Full programmatic window creation in setup()** (if config-based + post-config proves unreliable):

```rust
// Source: https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindowBuilder.html
use tauri::WebviewWindowBuilder;
use tauri::WebviewUrl;

let pill = WebviewWindowBuilder::new(
    app,
    "pill",
    WebviewUrl::App("pill.html".into()),
)
.title("")
.inner_size(120.0, 40.0)
.transparent(true)
.decorations(false)
.always_on_top(true)
.skip_taskbar(true)
.focusable(false)       // KEY: prevents focus steal — desktop only
.focused(false)         // don't focus on creation
.visible(false)         // hidden until first recording
.resizable(false)
.build()?;
```

### Pattern 3: Emitting State Events to the Pill

**What:** Reuse existing pipeline state transition points in lib.rs hotkey handler to emit pill state events.
**When to use:** Whenever pipeline transitions — same locations as `tray::set_tray_state()` calls.

```rust
// src-tauri/src/lib.rs — hotkey handler, ShortcutState::Pressed
// Source: https://v2.tauri.app/develop/calling-frontend/
use tauri::Emitter;
use serde::Serialize;

#[derive(Clone, Serialize)]
struct PillState {
    state: String, // "recording" | "processing" | "idle"
}

// On IDLE -> RECORDING:
app.emit_to("pill", "pill-state", PillState { state: "recording".into() }).ok();
app.emit_to("pill", "pill-show", ()).ok();

// On RECORDING -> PROCESSING (in hotkey handler):
app.emit_to("pill", "pill-state", PillState { state: "processing".into() }).ok();

// In pipeline.rs reset_to_idle() — BEFORE tray update:
app.emit_to("pill", "pill-state", PillState { state: "idle".into() }).ok();
app.emit_to("pill", "pill-hide", ()).ok();

// In pipeline.rs error paths (no-speech, transcription error):
app.emit_to("pill", "pill-error", ()).ok(); // triggers error flash then hide
```

### Pattern 4: RMS Level Streaming to Pill

**What:** A background task reads the audio buffer ~30 times/second, computes RMS of the last N samples, emits to pill window.
**When to use:** During RECORDING state only. Stop the loop when transitioning away from RECORDING.

```rust
// src-tauri/src/pill.rs — RMS streaming
// Source: derived from audio.rs try_lock() pattern (Phase 02 established)
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub fn start_level_stream(
    app: tauri::AppHandle,
    audio: Arc<crate::audio::AudioCapture>,
    active: Arc<AtomicBool>,
) {
    tauri::async_runtime::spawn(async move {
        while active.load(Ordering::Relaxed) {
            // Sample last 512 samples (~32ms at 16kHz) for RMS
            let rms = if let Ok(buf) = audio.buffer.try_lock() {
                let window = buf.len().min(512);
                if window > 0 {
                    let tail = &buf[buf.len() - window..];
                    let mean_sq: f32 = tail.iter().map(|&s| s * s).sum::<f32>() / window as f32;
                    mean_sq.sqrt()
                } else {
                    0.0
                }
            } else {
                0.0
            };

            // Normalize: typical speech RMS 0.01-0.1, scale to 0.0-1.0
            let normalized = (rms * 10.0).min(1.0);
            app.emit_to("pill", "pill-level", normalized).ok();

            // ~30fps
            tokio::time::sleep(std::time::Duration::from_millis(33)).await;
        }
    });
}
```

### Pattern 5: React Pill Component Structure

**What:** Separate Vite entry point for the pill window (pill.html). The pill renders Pill.tsx as root.
**When to use:** Multi-window Tauri apps need separate entry points per window.

```typescript
// src/Pill.tsx — pill window root component
// Source: @tauri-apps/api getCurrentWebviewWindow listen API
import { useEffect, useRef, useState } from 'react';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { FrequencyBars } from './components/FrequencyBars';

type PillDisplayState = 'hidden' | 'recording' | 'processing' | 'success' | 'error';

export function Pill() {
  const [displayState, setDisplayState] = useState<PillDisplayState>('hidden');
  const [level, setLevel] = useState(0);
  const appWindow = getCurrentWebviewWindow();

  useEffect(() => {
    const unlisten: Array<() => void> = [];

    appWindow.listen<string>('pill-state', (e) => {
      setDisplayState(e.payload as PillDisplayState);
    }).then(u => unlisten.push(u));

    appWindow.listen<number>('pill-level', (e) => {
      setLevel(e.payload);
    }).then(u => unlisten.push(u));

    appWindow.listen('pill-show', () => {
      // opacity transition handled via CSS class
      setDisplayState('recording');
    }).then(u => unlisten.push(u));

    appWindow.listen('pill-hide', () => {
      // trigger fade-out, then hide after transition
      setDisplayState('hidden');
    }).then(u => unlisten.push(u));

    appWindow.listen('pill-error', () => {
      setDisplayState('error');
      setTimeout(() => setDisplayState('hidden'), 500);
    }).then(u => unlisten.push(u));

    return () => unlisten.forEach(u => u());
  }, []);

  // ... render pill based on displayState + level
}
```

### Pattern 6: Pill Dragging with Position Persistence

**What:** Pill is draggable without activating the window. Use `startDragging()` on mousedown, save position to store on mouseup.
**When to use:** Any always-on-top overlay that must be repositionable without stealing focus.

```typescript
// src/Pill.tsx — drag handling
// Source: https://v2.tauri.app/reference/javascript/api/namespacewindow/
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { PhysicalPosition } from '@tauri-apps/api/window';
import { load } from '@tauri-apps/plugin-store';

const appWindow = getCurrentWebviewWindow();

async function handleMouseDown(e: React.MouseEvent) {
  e.preventDefault();
  await appWindow.startDragging(); // initiates OS-level window drag
}

async function handleMouseUp() {
  // Save position after drag completes
  const pos = await appWindow.outerPosition();
  const store = await load('settings.json', { autoSave: false });
  await store.set('pill-position', { x: pos.x, y: pos.y });
  await store.save();
}
```

**Position restore on startup (in Rust setup(), after creating pill window):**

```rust
// Source: https://v2.tauri.app/plugin/store/
use tauri_plugin_store::StoreExt;

if let Some(pill) = app.get_webview_window("pill") {
    let store = app.store("settings.json")?;
    if let Some(pos) = store.get("pill-position") {
        if let (Some(x), Some(y)) = (pos.get("x").and_then(|v| v.as_i64()),
                                      pos.get("y").and_then(|v| v.as_i64())) {
            let _ = pill.set_position(tauri::PhysicalPosition::new(x as i32, y as i32));
        }
    }
}
```

### Pattern 7: Animated Processing Border (CSS @property)

**What:** CSS conic-gradient border that rotates using `@property` for smooth animation. WebView2 (Chromium-based) supports `@property` and `conic-gradient` fully.
**When to use:** Processing state — pill border animates while Whisper runs.

```css
/* Source: https://ibelick.com/blog/create-animated-gradient-borders-with-css */
/* Verified: WebView2 uses Chromium, which fully supports @property */

@property --border-angle {
  syntax: "<angle>";
  initial-value: 0deg;
  inherits: false;
}

.pill-processing {
  border: 2px solid transparent;
  background:
    linear-gradient(#1a1a1a, #1a1a1a) padding-box,
    conic-gradient(from var(--border-angle), #6366f1, #a855f7, #06b6d4, #6366f1) border-box;
  border-radius: 9999px;
  animation: border-spin 2s linear infinite;
}

@keyframes border-spin {
  to { --border-angle: 360deg; }
}
```

### Anti-Patterns to Avoid

- **Using `focus: false` in tauri.conf.json alone:** Config-only focus prevention was broken (issue #11566). Use `focusable(false)` in the Rust `WebviewWindowBuilder`.
- **Using `data-tauri-drag-region` on a no-focus window:** `data-tauri-drag-region` requires window focus on Windows (#11605). Use programmatic `startDragging()` instead.
- **Using `app.emit()` (global) instead of `app.emit_to("pill", ...)`:** Global emit sends to ALL windows. The settings window would also receive pill events. Always use `emit_to` with the window label.
- **Locking `audio.buffer` with `lock()` in the RMS loop:** Use `try_lock()` — the audio callback runs on a background thread and holds the lock briefly. `lock()` can deadlock (same pitfall noted in Phase 02).
- **Creating the pill window inside a sync Tauri command:** Window creation via `WebviewWindowBuilder::new()` in a synchronous command handler deadlocks on Windows. Create in `setup()` or use `tauri::async_runtime::spawn`.
- **Streaming audio levels to pill during PROCESSING state:** RMS level streaming should stop when transitioning RECORDING -> PROCESSING. The audio buffer is no longer being filled, and the bars should go static/disappear per the spec.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Pill position persistence | Custom file-based storage | `tauri-plugin-store` (already used) | Already handles atomic saves, async API, JSON serialization |
| Animated gradient border | SVG or Canvas border animation | CSS `@property` + `conic-gradient` | Simpler, GPU-accelerated, works natively in WebView2 |
| Window drag without focus | Custom mouse-move event tracking | `appWindow.startDragging()` | OS-level drag — correct, reliable, no reinventing |
| Audio level from browser | Web Audio API AnalyserNode | Rust RMS from `audio.buffer` | Audio is already in Rust; no round-trip needed |
| Pill show/hide animation | JS-driven opacity manipulation | CSS `transition: opacity` + class toggle | Simpler, composited by GPU, no frame jank |

**Key insight:** Most of the "custom" work in this phase is wiring existing Rust state to an existing React pattern. The overlay mechanism (Tauri window), audio data (audio.buffer), state machine (PipelineState), and persistence (tauri-plugin-store) all already exist.

---

## Common Pitfalls

### Pitfall 1: focusable(false) May Still Activate Window on Drag
**What goes wrong:** Even with `focusable(false)`, initiating a window drag via `data-tauri-drag-region` can activate the window on Windows, stealing focus from the dictation target.
**Why it happens:** `data-tauri-drag-region` relies on an internal Tauri mechanism that may briefly activate the window to start the drag (issue #10767, #11605).
**How to avoid:** Use `appWindow.startDragging()` from a `mousedown` event listener on the pill element instead of `data-tauri-drag-region`. This routes the drag through the OS window manager without the activation step.
**Warning signs:** During drag, the dictation target (Notepad, VS Code, Chrome) loses its cursor blink or selection highlight.

### Pitfall 2: RMS Level Loop Not Stopped on RECORDING -> PROCESSING Transition
**What goes wrong:** The 30fps RMS emitter keeps running into PROCESSING state, reading a now-static buffer, and sending values to the pill visualizer which should show the processing animation instead.
**Why it happens:** The RMS loop's `AtomicBool` stop flag isn't set when the recording ends.
**How to avoid:** Store the stop flag `Arc<AtomicBool>` in managed state or as part of `AudioCapture`. Set it to `false` in the same hotkey handler block that transitions RECORDING -> PROCESSING.
**Warning signs:** Bars still moving during the processing border animation.

### Pitfall 3: Pill Window Missing from capabilities/default.json
**What goes wrong:** Tauri 2's capability system requires each window to be explicitly listed in capability JSON files. If "pill" is not in the `windows` array, the pill window gets no permissions and `app.emit_to("pill", ...)` silently fails (the webview can't receive events).
**Why it happens:** Capabilities are opt-in per window in Tauri 2. The current `default.json` only lists `"windows": ["settings"]`.
**How to avoid:** Add `"pill"` to the `windows` array in both `capabilities/default.json` and `capabilities/desktop.json`.
**Warning signs:** Pill shows but never updates state or level. No JS errors — events just don't arrive.

### Pitfall 4: Transparent Window + Non-Zero Background Color = Not Transparent
**What goes wrong:** Setting `transparent: true` in tauri.conf.json but leaving the pill's body/root div with a solid CSS background color means the window IS transparent at the OS level, but the React renders a solid rectangle.
**Why it happens:** `transparent` only tells the OS to composite the window's alpha channel. The DOM content must also use `rgba` colors with alpha for actual see-through.
**How to avoid:** The pill root element must have `background: transparent` (no Tailwind `bg-*` class on the root). The pill shape itself uses `bg-black/80` (Tailwind opacity modifier) or `rgba(0,0,0,0.8)`.
**Warning signs:** Pill appears as a solid black rectangle instead of a see-through pill shape.

### Pitfall 5: tauri-plugin-store Rust API is Async — Can't Use in Synchronous setup() for Position Restore
**What goes wrong:** Trying to call `store.get(...)` inside the synchronous `setup()` closure blocks when the store isn't loaded yet, or the async API doesn't exist in the sync context.
**Why it happens:** `tauri-plugin-store` Rust API requires async context for reads. `setup()` is synchronous (same pitfall as Phase 01-03 with `read_saved_hotkey`).
**How to avoid:** Use `std::fs` + `serde_json` to read the store JSON directly for position restore in setup (same pattern as `read_saved_hotkey`), OR emit a `pill-restore-position` event from frontend JS after the pill window loads and the JS store API resolves.
**Warning signs:** Position not restored on startup; Rust compiler error about async in sync context.

### Pitfall 6: WS_EX_NOACTIVATE vs focusable(false) — Don't Conflate
**What goes wrong:** STATE.md notes "Win32 WS_EX_NOACTIVATE required — config alone confirmed broken." This referred to the `focus: false` config key which was broken (issue #11566). The Rust builder `focusable(false)` is different and was added as the proper fix.
**Why it happens:** Two separate mechanisms: config JSON `focus` (now fixed for initial focus) vs. builder method `focusable` (prevents window from ever receiving activation).
**How to avoid:** Use `WebviewWindowBuilder::focusable(false)` first. Only add the Win32 `WS_EX_NOACTIVATE` hack via `windows-rs` if `focusable(false)` proves insufficient after testing against all four target apps (Notepad, VS Code, Chrome, Word).
**Warning signs:** Dictation target loses focus when pill appears or is dragged.

---

## Code Examples

Verified patterns from official sources:

### Tauri Event Emission to Specific Window (Rust)
```rust
// Source: https://v2.tauri.app/develop/calling-frontend/
use tauri::Emitter;
use serde::Serialize;

#[derive(Clone, Serialize)]
struct PillLevelPayload {
    level: f32, // 0.0 - 1.0
}

// Emit to pill window only (not settings window):
app.emit_to("pill", "pill-level", PillLevelPayload { level: 0.42 }).ok();

// Emit to pill window with unit payload:
app.emit_to("pill", "pill-show", ()).ok();
```

### Listening to Events in React (TypeScript)
```typescript
// Source: https://v2.tauri.app/develop/calling-frontend/
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

const appWindow = getCurrentWebviewWindow();

useEffect(() => {
  let unlisten: (() => void) | undefined;

  appWindow.listen<number>('pill-level', (event) => {
    setLevel(event.payload);
  }).then(fn => { unlisten = fn; });

  return () => { unlisten?.(); };
}, []);
```

### WebviewWindowBuilder for Overlay Window (Rust)
```rust
// Source: https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindowBuilder.html
use tauri::{WebviewWindowBuilder, WebviewUrl};

// In setup() — creates pill window that never steals focus
let _pill = WebviewWindowBuilder::new(
    app,
    "pill",
    WebviewUrl::App("pill.html".into()),
)
.title("")
.inner_size(120.0, 40.0)
.transparent(true)
.decorations(false)
.always_on_top(true)
.skip_taskbar(true)
.focusable(false)
.focused(false)
.visible(false)
.resizable(false)
.build()?;
```

### Position Persistence via tauri-plugin-store (TypeScript)
```typescript
// Source: https://v2.tauri.app/plugin/store/
import { load } from '@tauri-apps/plugin-store';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

const STORE_KEY = 'pill-position';
const appWindow = getCurrentWebviewWindow();

async function savePosition() {
  const pos = await appWindow.outerPosition();
  const store = await load('settings.json', { autoSave: false });
  await store.set(STORE_KEY, { x: pos.x, y: pos.y });
  await store.save();
}

async function loadPosition() {
  const store = await load('settings.json', { autoSave: false });
  const saved = await store.get<{ x: number; y: number }>(STORE_KEY);
  if (saved) {
    await appWindow.setPosition({ type: 'Physical', x: saved.x, y: saved.y });
  } else {
    // Default: bottom center — calculate from screen size
    // Use PhysicalPosition from primary monitor center
  }
}
```

### RMS Calculation from Buffer (Rust)
```rust
// Source: derived from audio.rs existing try_lock() pattern
fn compute_rms(buf: &[f32], window: usize) -> f32 {
    if buf.is_empty() { return 0.0; }
    let n = buf.len().min(window);
    let tail = &buf[buf.len() - n..];
    let mean_sq: f32 = tail.iter().map(|&s| s * s).sum::<f32>() / n as f32;
    (mean_sq.sqrt() * 10.0).min(1.0) // normalize: speech RMS ~0.01-0.1 → 0.0-1.0
}
```

### Frequency Bar Component (TypeScript/React)
```tsx
// Frequency bars using CSS transitions — no canvas needed for 15 bars
interface FrequencyBarsProps {
  level: number; // 0.0 - 1.0 overall RMS
  count?: number; // default 15
}

// Each bar gets a pseudo-random multiplier to simulate frequency variation
// The true approach: emit per-band levels from Rust; the approximate approach:
// use a set of fixed frequency multipliers applied to overall RMS
const BAND_MULTIPLIERS = [0.3, 0.5, 0.7, 0.9, 1.0, 0.95, 0.85, 1.0, 0.9, 0.75, 0.6, 0.8, 0.65, 0.45, 0.3];

export function FrequencyBars({ level, count = 15 }: FrequencyBarsProps) {
  return (
    <div className="flex items-end gap-[2px] h-6">
      {BAND_MULTIPLIERS.slice(0, count).map((mult, i) => (
        <div
          key={i}
          className="w-[3px] rounded-full bg-indigo-400 transition-all duration-75"
          style={{ height: `${Math.max(2, level * mult * 100)}%` }}
        />
      ))}
    </div>
  );
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `focus: false` in tauri.conf.json | `WebviewWindowBuilder::focusable(false)` in Rust | Nov 2024 (PR #11569, #7722) | Config approach was broken; builder method works reliably |
| `data-tauri-drag-region` for all drag | `appWindow.startDragging()` programmatically | Ongoing — issue #11605 | Drag-region breaks on unfocused windows on Windows |
| Web Audio API AnalyserNode for visualizers | Rust-side RMS + Tauri events | N/A (architecture decision) | Audio is in Rust — no need to pipe to browser context |
| `app.emit_all()` for backend events | `app.emit_to("label", ...)` for targeted events | Tauri 2.x | emit_to is the correct multi-window pattern; emit_all hits all windows |

**Deprecated/outdated:**
- `focus: false` in tauri.conf.json: Still works for initial focus state but WAS broken for overlay use case. Do not rely on it for no-activate behavior — use `focusable(false)` in builder.
- `app.emit_all()`: Not deprecated but incorrect for multi-window apps where only specific windows should receive events.
- `HasRawWindowHandle` / `has_raw_window_handle()`: Deprecated in raw-window-handle crate in favor of `HasWindowHandle`. Use `window_handle()` trait if raw HWND access is needed.

---

## Open Questions

1. **Does `focusable(false)` fully prevent focus steal during drag on Windows?**
   - What we know: `focusable(false)` was added as the correct API for no-activate behavior. STATE.md concern about WS_EX_NOACTIVATE referred to the old config approach which is now fixed. The drag issue (#11605) is separate.
   - What's unclear: Whether `appWindow.startDragging()` on a `focusable(false)` window triggers a focus steal on Windows specifically. This needs empirical verification against target apps.
   - Recommendation: Implement with `focusable(false)` + `startDragging()`. Test against Notepad, VS Code, Chrome, Word. If focus steal is observed during drag, add Win32 `WS_EX_NOACTIVATE` via windows-rs crate.

2. **Separate Vite entry point (pill.html) vs. React Router for the pill window**
   - What we know: The settings window uses a single `index.html` / `App.tsx`. The pill needs a completely different UI with no settings components.
   - What's unclear: Whether the existing Vite config supports multiple entry points or if a second HTML file needs to be added.
   - Recommendation: Add `pill.html` as a second Vite entry point in `vite.config.ts` using `build.rollupOptions.input`. This is the standard Tauri multi-window pattern.

3. **Per-band frequency data vs. RMS-only for visualizer realism**
   - What we know: The audio buffer is 16kHz mono PCM. FFT is needed for true per-band frequency data. The user specified "frequency bars style" but the audio is captured for transcription, not visualization.
   - What's unclear: Whether simulated per-band variation (fixed multipliers × overall RMS) looks realistic enough, or if a proper FFT is needed.
   - Recommendation: Start with simulated variation (fixed multipliers) — simpler, no FFT crate needed. The bars will pulse together with the voice level. If it looks too uniform, add a Rust FFT (rustfft crate) to compute ~15 frequency bands.

4. **Default position calculation (bottom center)**
   - What we know: The pill defaults to bottom center of screen. Getting screen size from Rust setup() requires the monitor API. Getting it from JS is straightforward.
   - What's unclear: Whether to compute default position in Rust (setup, before pill is shown) or in JS (pill component, on first show).
   - Recommendation: Compute in JS using `window.screen.width`/`height` on first load if no stored position exists. Simpler than Rust monitor API.

---

## Integration Checklist

Items that must be touched to implement this phase (for planner reference):

**Rust side:**
- [ ] `tauri.conf.json`: Add pill window definition (or create programmatically in setup)
- [ ] `src-tauri/capabilities/default.json`: Add `"pill"` to `windows` array
- [ ] `src-tauri/capabilities/desktop.json`: Add `"pill"` to `windows` array (for global-shortcut permissions)
- [ ] `src-tauri/src/lib.rs`: Create pill window in `setup()` with `WebviewWindowBuilder::focusable(false)`
- [ ] `src-tauri/src/lib.rs`: Add `emit_to("pill", ...)` calls alongside existing `tray::set_tray_state()` calls
- [ ] `src-tauri/src/pipeline.rs`: Add pill events in `reset_to_idle()` and error paths
- [ ] `src-tauri/src/pill.rs` (new): RMS level streaming loop
- [ ] `src-tauri/Cargo.toml`: Add `tauri::Emitter` usage (already imported in existing code)

**Frontend side:**
- [ ] `vite.config.ts`: Add `pill.html` as second Rollup entry point
- [ ] `pill.html`: New HTML file for pill window
- [ ] `src/pill-main.tsx`: New entry point rendering `<Pill />`
- [ ] `src/Pill.tsx`: Main pill component with state machine and event listeners
- [ ] `src/components/FrequencyBars.tsx`: ~15 animated vertical bars component

---

## Sources

### Primary (HIGH confidence)
- https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindowBuilder.html — `focusable()`, `transparent()`, `decorations()`, `always_on_top()`, `skip_taskbar()`, `focused()`, `visible()` builder methods
- https://v2.tauri.app/develop/calling-frontend/ — `app.emit_to()` Rust API, `getCurrentWebviewWindow().listen()` TypeScript API with typed payloads
- https://v2.tauri.app/plugin/store/ — `store.set()`, `store.get()`, `store.save()` for position persistence
- https://ibelick.com/blog/create-animated-gradient-borders-with-css — `@property` + `conic-gradient` animated border (WebView2/Chromium compatible)

### Secondary (MEDIUM confidence)
- https://github.com/tauri-apps/tauri/issues/11566 — Confirmed: config `focus: false` was broken, fixed via builder API (PR #11569, Nov 2024)
- https://github.com/tauri-apps/tauri/issues/11605 — Confirmed: `data-tauri-drag-region` fails on unfocused windows; `startDragging()` is the workaround
- https://github.com/tauri-apps/tauri/discussions/7951 — Official maintainer recommends `decorations: false` + `ignoreCursorEvents: true` + `focus: false` for overlay notification windows
- WebSearch: `focusable` method documented in WebviewWindowBuilder — adds no-activate behavior for desktop-only

### Tertiary (LOW confidence)
- STATE.md note about WS_EX_NOACTIVATE: Was accurate for the old config approach; `focusable(false)` builder method is the current fix. Direct Win32 manipulation may still be needed but requires empirical testing to confirm.
- Per-band frequency bars using fixed multipliers: Unverified whether this looks acceptable — needs UI testing.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries already in project, APIs verified via official docs
- Architecture: HIGH — patterns derived from existing project code + official Tauri docs
- Focus prevention: MEDIUM — `focusable(false)` verified in API docs; drag + no-focus behavior on Windows needs empirical testing
- Pitfalls: HIGH — derived from verified GitHub issues + Phase 01-03 established patterns

**Research date:** 2026-02-28
**Valid until:** 2026-03-28 (Tauri 2 active development — check for focusable API changes)
