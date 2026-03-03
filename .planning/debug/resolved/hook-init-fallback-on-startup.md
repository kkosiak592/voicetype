---
status: resolved
trigger: "hook-init-fallback-on-startup"
created: 2026-03-03T00:00:00
updated: 2026-03-03T00:25:00
---

## Current Focus

hypothesis: CONFIRMED (refined) — Tauri events have no queue. The first fix (emit from setup()) failed because setup() emits BEFORE the frontend's listen() round-trip completes. The event is silently dropped (Tauri issue #3484). The fix requires coordinating both sides: frontend must signal readiness, backend must emit only after listener is registered.
test: Two-flag handshake (SetupComplete + FrontendReady) implemented. cargo check + tsc clean.
expecting: No "Hook unavailable" warning on startup.
next_action: User runs cargo tauri dev and verifies.

## Symptoms

expected: On initial launch with Ctrl+Win hotkey, the low-level keyboard hook should install successfully without showing "Hook unavailable — using standard shortcut fallback"
actual: On initial dev build launch, the settings page shows "Hook unavailable — using standard shortcut fallback" in orange text under the Ctrl+Win hotkey. After switching hotkey to Ctrl+Shift+Space (warning disappears) and then switching back to Ctrl+Win, the warning is gone AND the hook actually works.
errors: "Hook unavailable — using standard shortcut fallback" displayed in UI on initial launch
reproduction: 1) cargo tauri dev  2) Open settings  3) See "Hook unavailable — using standard shortcut fallback" under Ctrl+Win hotkey  4) Switch to Ctrl+Shift+Space  5) Switch back to Ctrl+Win  6) Warning gone, hook works
started: On initial dev build launch. The cycling fix makes the hook work within the same process — so it's not a fundamental hook installation failure, but a timing/ordering issue in the initial setup path.

## Eliminated

- hypothesis: Emit hook-status-changed from setup() after hook routing; listen in App.tsx (v1 fix)
  evidence: User confirmed NOT FIXED. Event emitted from setup() is dropped — frontend JS listener not registered yet when setup() emits. Tauri events have no queue.
  timestamp: 2026-03-03T00:15:00

## Evidence

- timestamp: 2026-03-03T00:01:00
  checked: lib.rs Builder registration of HookAvailable
  found: HookAvailable is registered on Builder with initial value FALSE — `AtomicBool::new(false)`. Comment says "Starts false; setup() updates via the shared Arc."
  implication: The frontend CAN query get_hook_status before setup() completes because webview2 COM init pumps Win32 messages. If it does, it sees false.

- timestamp: 2026-03-03T00:02:00
  checked: App.tsx loadSettings() sequence
  found: loadSettings() is called on mount. It awaits check_first_run, then getStore(), then get_engine(), THEN invokes get_hook_status. This is all sequential async I/O. The full chain runs in a single useEffect on mount.
  implication: By the time get_hook_status is called, setup() may or may not have run. setup() is the ONLY place HookAvailable gets set to true.

- timestamp: 2026-03-03T00:03:00
  checked: setup() in lib.rs lines 1583-1911
  found: setup() calls keyboard_hook::install() and if successful stores true into hook_available. But setup() ALSO does many other heavy async things AFTER the hotkey installation: audio capture stream initialization, Parakeet model loading (blocking), Whisper model loading (blocking + GPU detection). These happen AFTER hook installation but within the same setup() closure.
  implication: setup() itself is synchronous and linear. The hook IS installed at line ~1742-1748, then hook_available.store(true) immediately after. So hook_available is true early in setup().

- timestamp: 2026-03-03T00:04:00
  checked: When does the frontend's useEffect loadSettings() run vs when setup() fires?
  found: The Tauri documentation and the existing comment in lib.rs (line 1481-1483) explicitly state: "Tauri creates webviews during run() and the webview2 COM init pumps the Win32 message loop, which lets the frontend call check_first_run/list_models before setup() even fires." This means the frontend's useEffect can fire BEFORE setup() runs.
  implication: If loadSettings() completes its await chain and reaches get_hook_status before setup() has run keyboard_hook::install(), hook_available is still false → warning shows.

- timestamp: 2026-03-03T00:05:00
  checked: rebind_hotkey() path when cycling hotkeys
  found: rebind_hotkey() tears down old hook, then reinstalls. After the install it does NOT update HookAvailable (the hook_status.0.load check at line 559 handles the "not yet installed" case, but when hook_status is already false: it installs and sets hook_status.0.store(true)). Then App.tsx onHotkeyChange callback at line 160-161 calls get_hook_status again: `invoke<boolean>('get_hook_status').then(setHookAvailable)`. This re-query happens AFTER rebind_hotkey returns, by which time the hook IS installed and hook_available is true.
  implication: The cycle fix works because: rebind installs the hook + sets HookAvailable=true, then the frontend immediately re-queries get_hook_status. Initial load doesn't do a re-query after setup() completes.

## Resolution

root_cause: Two-layer race condition:
  1. Frontend calls get_hook_status during loadSettings() before setup() has run (webview2 COM init pumps Win32 messages allowing IPC before setup fires). Gets false incorrectly.
  2. The event-based fix (emit from setup()) also fails: Tauri events have no queue. If setup() emits before the frontend's listen() IPC round-trip completes, the event is silently dropped (Tauri issue #3484). There is no way to emit reliably from setup() because the webview JS may not have its listener registered yet.

fix: Two-flag handshake (SetupComplete + FrontendReady):
  - Frontend: registers listen("hook-status-changed"), then after listen() resolves (listener guaranteed registered), calls invoke("notify_frontend_ready").
  - Backend: notify_frontend_ready command sets FrontendReady=true; if SetupComplete is already true, emits hook-status-changed immediately.
  - Backend: setup() sets SetupComplete=true after hook routing; if FrontendReady is already true, emits immediately.
  - The emit fires exactly once, whichever side completes last, guaranteeing the listener is registered when the event arrives.
  - Also removed the get_hook_status call from loadSettings() to eliminate the stale read.
  - hookAvailable initialized to true (no warning flash before the handshake completes).

verification: cargo check passes clean; tsc --noEmit passes clean. Runtime verified: user tested "cargo tauri dev", confirmed "Hook unavailable" warning no longer appears on startup. The hotkey works correctly from initial launch.

files_changed:
  - src-tauri/src/lib.rs (add SetupComplete/FrontendReady state; add notify_frontend_ready command; update setup() emit logic)
  - src/App.tsx (call notify_frontend_ready after listen() resolves; remove get_hook_status from loadSettings)
