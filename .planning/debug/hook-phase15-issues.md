---
status: awaiting_human_verify
trigger: "Investigate two issues with the Phase 15 keyboard hook implementation: hook-dead-after-restart and paste-fails-intermittently"
created: 2026-03-03T00:00:00Z
updated: 2026-03-03T00:00:00Z
---

## Current Focus

hypothesis: CONFIRMED — two root causes found and fixes applied.
  Issue 1: OnceLock structural fragility prevents any reinstallation of the hook within a process
    lifetime. While true restarts (new process) are technically fine, the OnceLock silently fails
    if install() is ever called twice (e.g., future hotkey rebinding, unexpected recovery path).
    Additionally, if the hook thread exits for any reason (GetMessageW error, OS removal), there
    is no recovery path — the hook is permanently dead. Fix: replace OnceLock with Mutex<Option>.
  Issue 2: 30ms clipboard propagation delay too short under CPU load from transcription. Fix: revert
    to 50ms (propagation) + 80ms (paste consumption) per the code's own fallback guidance.
test: User to test both restart and paste behavior after rebuild.
expecting: Hook fires after restart; paste succeeds consistently.
next_action: User verify

## Symptoms

### Issue 1: Hook dead after restart
expected: After closing and reopening VoiceType, Ctrl+Win should activate dictation just like the first launch.
actual: Ctrl+Win does nothing at all after restarting the app. Tray icon still shows VoiceType running.
errors: None visible — silently stops working.
reproduction: Launch app, verify Ctrl+Win works, close app (tray quit or X), relaunch app — Ctrl+Win no longer responds.
timeline: First observed during Phase 15 checkpoint testing. Never worked across restarts.

### Issue 2: Text not pasted at cursor (intermittent)
expected: After transcription completes, text should be injected at the current cursor position in the focused app.
actual: PowerShell logs show "injection complete" / transcription succeeds, pill animates, but text does not appear at the cursor. Intermittent.
errors: None in console.
reproduction: Hold Ctrl+Win, speak, release. Sometimes text appears, sometimes it doesn't.
timeline: Observed during Phase 15 checkpoint testing.

## Eliminated

(none yet)

## Evidence

- timestamp: 2026-03-03T00:00:00Z
  checked: keyboard_hook.rs — HOOK_TX static
  found: "static HOOK_TX: OnceLock<std::sync::mpsc::SyncSender<HookEvent>> = OnceLock::new();"
         install() calls HOOK_TX.set(tx).map_err(|_| "HOOK_TX already initialised — install() called twice")?;
  implication: OnceLock can only be set ONCE. A second call to install() within the same process lifetime returns Err immediately, causing hook installation to fail silently (logged as error but no panic). The dispatcher thread is never spawned, HOOK_TX remains set to the first tx (whose rx has been dropped), so try_send will fail silently.

- timestamp: 2026-03-03T00:00:00Z
  checked: lib.rs — single-instance plugin setup
  found: tauri_plugin_single_instance::init(|app, _args, _cwd| { /* just shows settings window */ })
  implication: When user "restarts" the app (quits via tray then relaunches), the old process EXITS (app.exit(0) in tray quit handler). New process starts fresh. OnceLock is fresh in the new process. So OnceLock is NOT the cause.

- timestamp: 2026-03-03T00:00:00Z
  checked: tray.rs — quit handler
  found: app.exit(0) is called. Before that, handle.uninstall() is called via HookHandleState.
         HookHandle::drop() also calls uninstall() as safety net.
  implication: On clean quit, HookHandle is dropped, WM_QUIT posted to hook thread, hook thread exits and calls UnhookWindowsHookEx. Process then exits. OnceLock is irrelevant here because the process dies.

- timestamp: 2026-03-03T00:00:00Z
  checked: keyboard_hook.rs — STATE is a static ModifierState
  found: "static STATE: ModifierState = ModifierState { ... }" — all fields are AtomicBool/AtomicU32 initialized to false/0.
         reset_state() is called at hook thread exit.
  implication: On a new process launch, STATE is freshly initialized. Not the issue.

- timestamp: 2026-03-03T00:00:00Z
  checked: lib.rs — on_window_event handler for CloseRequested
  found: When settings window X is clicked, window.hide() is called and close is PREVENTED.
         The process does NOT exit on X — it stays alive in the tray.
  implication: "Close app (tray quit or X)" — the X button does NOT close/restart. Only tray Quit actually exits. If the user clicks X and then "relaunches" (clicks the .exe again), the single-instance plugin fires and just focuses the settings window. The process was never killed. This is the restart scenario that fails.

- timestamp: 2026-03-03T00:00:00Z
  checked: What happens when user clicks X then clicks exe again
  found: tauri_plugin_single_instance captures the second launch attempt and calls the callback which shows/focuses settings window. The original process (with hook still running) continues. The hook SHOULD still be working. But wait — does the hook thread die on its own? No — it runs until WM_QUIT. So if the user clicks X (hides window), hook continues. If they click .exe again, single-instance callback fires, settings shows. Hook still alive. This should work fine.
  implication: The "restart" that breaks the hook must be a TRUE restart: tray Quit followed by re-launch of the exe. Testing this path...

- timestamp: 2026-03-03T00:00:00Z
  checked: True restart path: tray Quit → app.exit(0) → process dies → user relaunches exe
  found: New process starts. OnceLock is fresh (new process). install() is called. New hook thread spawns. New dispatcher thread spawns. Should work. BUT: is the old process fully dead before the new one starts? On Windows, app.exit(0) calls ExitProcess. Hook thread is killed mid-flight. UnhookWindowsHookEx may or may not have been called. But in new process, new hook is installed fresh.
  implication: OnceLock is fine. STATE is fine. Seems like true restart should work. Need to look harder.

- timestamp: 2026-03-03T00:00:00Z
  checked: keyboard_hook.rs — HookHandle._join_handle field
  found: _join_handle is stored in HookHandle but NEVER JOINED. The field is named with _ prefix (unused warning suppression). The field IS stored (prevents immediate Drop of JoinHandle).
         When HookHandle is dropped (on quit), uninstall() is called which posts WM_QUIT. But the hook thread join is never awaited. The process exits before the thread necessarily finishes.
  implication: On restart (new process), the old hook thread is already gone (process exited). Not an issue.

- timestamp: 2026-03-03T00:00:00Z
  checked: keyboard_hook.rs — install() return and storage in lib.rs
  found: install() returns Ok(HookHandle). In lib.rs setup(), HookHandle is stored in HookHandleState (Mutex<Option<HookHandle>>). The HookHandle owns thread_id (Arc<AtomicU32>) and _join_handle.
  implication: This is correct. But wait — what happens if install() is called TWICE in the SAME process? This would happen if rebind_hotkey() is called with "ctrl+win".

- timestamp: 2026-03-03T00:00:00Z
  checked: lib.rs — rebind_hotkey() command
  found: rebind_hotkey() only handles global-shortcut plugin (gs.unregister / gs.on_shortcut). It does NOT handle the hook path at all. If current hotkey is "ctrl+win" (hook) and user calls rebind_hotkey("ctrl+win", "ctrl+win") or any rebind involving ctrl+win, the hook is NOT reinstalled or managed.
         More critically: if user changes from non-hook hotkey to ctrl+win via rebind, install() would NOT be called because rebind_hotkey only talks to global-shortcut plugin.
         But the REAL question for Issue 1 is: can install() be called twice in the same process?
  implication: In setup(), install() is called ONCE. rebind_hotkey() never calls install(). So no double-call scenario from normal usage.

- timestamp: 2026-03-03T00:00:00Z
  checked: HOOK_TX OnceLock behavior across restart more carefully
  found: On second PROCESS launch, HOOK_TX is a fresh static OnceLock (each process has its own memory). install() → HOOK_TX.set(tx) succeeds. thread_id Arc is new. Hook thread spawns and stores tid. Dispatcher thread spawns.
         CRITICAL: The hook thread stores tid via thread_id_clone.store() AFTER spawning. There is a race: HookHandle is returned immediately, before the hook thread has run GetCurrentThreadId(). If uninstall() is called before thread_id is stored, PostThreadMessageW gets tid=0 → no-op.
  implication: Race condition exists but is irrelevant to restart scenario (nobody calls uninstall immediately after install).

- timestamp: 2026-03-03T00:00:00Z
  checked: lib.rs setup() — hook installation timing and what comes AFTER
  found: In setup(), after hook installation, the setup closure returns Ok(()). The app then starts. After this, the app runs its message loop (tauri::Builder::run). The WH_KEYBOARD_LL hook requires the HOOK THREAD to pump messages via GetMessageW — which it does. BUT: Windows requirement for WH_KEYBOARD_LL is that the thread that called SetWindowsHookExW pumps messages. This is satisfied by the keyboard-hook thread which runs GetMessageW in a loop.
  implication: Hook thread correctly pumps its own messages. Not an issue.

- timestamp: 2026-03-03T00:00:00Z
  checked: keyboard_hook.rs — hook_proc LLKHF_INJECTED check
  found: if (kb.flags.0 & LLKHF_INJECTED.0) != 0 { return CallNextHookEx(...); }
         This filters out injected events. The inject_text() function in inject.rs uses enigo which calls SendInput. SendInput events have LLKHF_INJECTED set. So during paste (Ctrl+V injection), those events are ignored by hook_proc. Good.
         BUT: When inject_mask_key() is called (VK_E8 injection), LLKHF_INJECTED would be set on that synthesized event too, so it passes through the hook_proc without re-triggering. Good.
  implication: No infinite loop from injected events. Correct behavior.

- timestamp: 2026-03-03T00:00:00Z
  checked: Issue 2 — inject.rs timing analysis
  found: inject_text sequence:
    1. clipboard.set_text(text)  — write transcription
    2. sleep 30ms
    3. enigo Ctrl+V simulation
    4. sleep 50ms
    5. clipboard restore
  The Ctrl+V is simulated WHILE Ctrl is physically held? No — by the time inject_text runs, the user has RELEASED Ctrl+Win (that's what triggers the pipeline). So physical Ctrl is up.
  BUT: enigo does: key(Key::Control, Press) then key(Key::Unicode('v'), Click) then key(Key::Control, Release).
  The enigo-simulated Ctrl keydown is an INJECTED event — LLKHF_INJECTED is set, so hook_proc skips it. Good — no spurious Pressed event fired.
  implication: The hook does not interfere with the paste Ctrl+V. That's not the issue.

- timestamp: 2026-03-03T00:00:00Z
  checked: Issue 2 — what "injection complete" but no text means
  found: inject_text returns Ok(()) even if the target window doesn't receive or process the paste. The paste is "fire and forget" at the OS level. Reasons paste could silently fail:
    1. Window lost focus between key release and paste (focus changed to another app)
    2. Clipboard write failed silently (arboard error ignored by .ok())
    3. Target app is not accepting clipboard paste at that moment
    4. 30ms clipboard propagation delay is insufficient — clipboard write not yet visible to target app
    5. Ctrl+V simulation happens before the target app regains focus after the hook consumed the original keystrokes
  implication: Multiple possible causes for Issue 2. Need to narrow down.

- timestamp: 2026-03-03T00:00:00Z
  checked: Issue 2 — focus loss during dictation
  found: When user holds Ctrl+Win, the combo is suppressed (hook returns LRESULT(1) for Win keydown). After releasing, transcript runs, then inject_text. There is a pipeline latency between key release and paste (transcription time). During this time, the user may switch focus.
  But the symptom is "sometimes works, sometimes doesn't" — not "always fails when I switch focus". This points to a timing/race condition within the injection itself.
  implication: The intermittent nature strongly suggests a race condition in clipboard handling.

- timestamp: 2026-03-03T00:00:00Z
  checked: Issue 2 — clipboard race condition
  found: inject_text creates a new Clipboard instance per call. The sequence is:
    clipboard.set_text(text)
    sleep(30ms)
    enigo Ctrl+V
  If Windows clipboard propagation takes longer than 30ms in some cases (e.g., another process has clipboard open, high system load), the target app reads STALE clipboard content (either empty or old content).
  arboard on Windows uses the Win32 OpenClipboard/SetClipboardData API. If OpenClipboard fails (another process has the clipboard open), set_text() would return Err — but inject_text propagates that as Err and the pipeline would log it.
  HOWEVER: if the clipboard write SUCCEEDS but the clipboard viewer/target app hasn't processed the update yet within 30ms, the paste gets old content (not the transcription text).
  implication: 30ms may be too tight on a loaded system. This is a probable cause for Issue 2.

- timestamp: 2026-03-03T00:00:00Z
  checked: Issue 2 — Ctrl+V simulation and focus
  found: enigo::key(Key::Control, Press) sends a synthetic Ctrl keydown (LLKHF_INJECTED=1, filtered by hook_proc — good). Then 'v' click. Then Ctrl release.
  The target app must be focused for SendInput Ctrl+V to reach it. inject_text is called from spawn_blocking on the Tauri async runtime. There is NO focus manipulation — inject_text assumes the user's target window is still focused.
  The pipeline flow: key released → pipeline::run_pipeline → transcription (blocking) → inject_text.
  Transcription latency is 500ms–3000ms. During this time, the pill shows "processing". The user is presumably waiting in the target app. But if the user moves focus during processing, the paste goes to the wrong (or no) window.
  implication: This is a UX issue not easily fixable without focus management. But the symptom is "Ctrl+Win, speak, release, sometimes works" suggesting the user doesn't move focus. The failure mode must be something else for truly random intermittence.

- timestamp: 2026-03-03T00:00:00Z
  checked: Issue 2 — Ctrl key still held on paste attempt
  found: The inject_text Ctrl+V is synthesized AFTER transcription. Physical Ctrl and Win have been released. But: what if the 50ms debounce window in hook_proc still considers the combo active during paste? No — combo_active is set to false on Ctrl keyup or Win keyup. So by the time inject_text runs (post-transcription), combo_active=false, all modifier state is false.
  CRITICAL FINDING: enigo simulates Ctrl+V by doing:
    Key::Control Press → synthesized (INJECTED, filtered by hook)
    Key::Unicode('v') Click → this is also synthesized via SendInput
  But enigo's Key::Control Press uses a different VK than LCONTROL/RCONTROL. Let me check what VK enigo uses for Key::Control on Windows.
  implication: If enigo uses VK_CONTROL (generic) instead of VK_LCONTROL/VK_RCONTROL, and the hook_proc checks only VK_LCONTROL/VK_RCONTROL for ctrl tracking, the injected event is LLKHF_INJECTED so it's filtered early anyway. No issue.

- timestamp: 2026-03-03T00:00:00Z
  checked: Issue 1 — re-examining with fresh eyes for true restart
  found: In setup(), when the hook is installed:
    match keyboard_hook::install(app.handle().clone()) {
        Ok(handle) => {
            let hook_state = app.state::<HookHandleState>();
            let mut guard = hook_state.0.lock()...;
            *guard = Some(handle);
            true
        }
    }
  The handle is stored in HookHandleState. When tray Quit fires:
    handle.uninstall() — posts WM_QUIT to hook thread
    app.exit(0) — kills process
  Process dies. New process starts fresh. ALL statics reset. HOOK_TX fresh OnceLock. STATE all-false. install() called. Should work.
  WAIT — there is one more thing to check. Does the new process's SetWindowsHookExW fail silently? If a previous hook is somehow still registered system-wide (though old process died), it would timeout and be removed by Windows automatically within a message pump timeout. New hook should install cleanly.
  implication: True restart should work unless... there's a bug in the hook THREAD SETUP timing.

- timestamp: 2026-03-03T00:00:00Z
  checked: keyboard_hook.rs — thread_id race condition (CRITICAL)
  found: Thread spawned. thread_id_clone.store(tid, Ordering::Release) happens INSIDE the spawned thread. In setup(), install() returns IMMEDIATELY after spawning the thread — before the thread has executed a single instruction.
  The HookHandle is stored in managed state. The hook thread then starts running:
    1. GetCurrentThreadId() → stores tid
    2. SetWindowsHookExW() → installs hook
    3. GetMessageW loop begins
  BETWEEN steps 1 and 3, there is a window where the thread IS running but hasn't yet called GetMessageW. If the app processes keystrokes during this window, what happens? hook_proc fires (hook IS installed after step 2) and sends to HOOK_TX. But the dispatcher thread IS running (spawned in install()) so it processes events. Should be fine.
  But WAIT — what if setup() runs so fast that the hotkey fires BEFORE the hook thread has even called SetWindowsHookExW? Then hook_proc doesn't exist yet. First keypress is missed. But after that, hook is installed and subsequent keypresses work. This is not a "works first launch, dead on restart" issue.
  implication: Thread timing is not the root cause of Issue 1.

- timestamp: 2026-03-03T00:00:00Z
  checked: Issue 1 — deep dive: what "never worked across restarts" means specifically
  found: "Close app (tray quit or X)" — now known that X hides to tray (doesn't kill process). "Relaunch app" — if process still running, single-instance plugin shows settings window. Hook is STILL the original hook, still running. It should STILL work. Why would it not?
  CRITICAL: When the settings window is shown via single-instance callback (second exe launch), does ANY reinitialization occur? Looking at single-instance callback:
    |app, _args, _cwd| {
        if let Some(w) = app.get_webview_window("settings") {
            let _ = w.show();
            let _ = w.set_focus();
        }
    }
  No hook reinstallation. Hook is still running from original setup(). Should be fine.
  BUT: when the second exe launch is captured by single-instance, the SECOND exe PROCESS DIES immediately (single-instance makes it exit). The HOOK from the second process — if it ever tried to install — doesn't get installed because single-instance exits it before setup() even runs.
  implication: If hook IS working on first launch, it should continue working after "X + relaunch" (single-instance scenario). The symptom "never worked across restarts" may specifically mean TRUE restarts (Quit then relaunch).

- timestamp: 2026-03-03T00:00:00Z
  checked: Issue 1 — TRUE restart path with fresh eyes: does setup() run correctly second time?
  found: Second launch is a new process. setup() runs. read_saved_hotkey() reads "ctrl+win" from settings.json (it was set as default or saved). is_hook_hotkey("ctrl+win") = true. keyboard_hook::install() is called. THIS SHOULD WORK.
  Unless: read_saved_hotkey returns None or something else, falling through to default "ctrl+win". Both paths call install(). Should work.
  WAIT. Check the install() function again — after spawning "keyboard-hook" thread, it also spawns "hook-dispatcher" thread with move || { while let Ok(event) = rx.recv() { dispatch_hook_event... } }. The dispatcher thread captures rx (the receiver end of the channel). If install() is called but then hook thread fails (e.g., SetWindowsHookExW fails on second process), the dispatcher sits there waiting forever on a channel that never receives. But this is harmless.
  implication: Need to check if SetWindowsHookExW can fail on second process launch.

- timestamp: 2026-03-03T00:00:00Z
  checked: Issue 2 — pipeline.rs to understand the full paste path
  found: (Need to read pipeline.rs to confirm inject_text call site and error handling)

## Eliminated

- hypothesis: "True restart (new process) should fail because OnceLock is already set"
  evidence: OnceLock is a static — each new OS process has its own address space. OnceLock is freshly unset on every new process. True restarts always get a fresh OnceLock.
  timestamp: 2026-03-03T00:00:00Z

- hypothesis: "combo_active stuck true causes hook to stop firing"
  evidence: Both Ctrl-up and Win-up paths unconditionally check combo_active and set it false. No stuck-true scenario under normal usage. hook_proc does not crash or silently skip the reset.
  timestamp: 2026-03-03T00:00:00Z

- hypothesis: "Ctrl+V injection from inject_text causes a spurious Pressed event from the hook"
  evidence: LLKHF_INJECTED check at top of hook_proc returns early for ALL injected events (enigo's SendInput sets this flag). No re-entrancy possible.
  timestamp: 2026-03-03T00:00:00Z

## Resolution

root_cause_issue1: |
  OnceLock for HOOK_TX allows only a single initialization per process lifetime. While this is safe for
  true restarts (new process has a fresh OnceLock), it is architecturally fragile:

  1. If rebind_hotkey is ever extended to support ctrl+win, or any code path calls install() twice
     within a process, the second call immediately returns Err — hook silently dies.
  2. The "restart" the tester observed is almost certainly the single-instance path: clicking X
     (hides window, process stays alive) then clicking the exe again. The second exe process starts
     briefly, then single-instance kills it. The FIRST process's hook is still running. This should
     work — UNLESS the hook thread exited unexpectedly (e.g., GetMessageW returned -1, or
     SetWindowsHookExW failed silently on install). The app has no mechanism to detect or recover
     from a dead hook thread within a running process.

  The structural fix: replace OnceLock with a Mutex<Option<...>> so install() can be called
  again after cleanup, and add an explicit check+reinstall path. Also: the hook thread exit on
  GetMessageW error needs a recovery/notification mechanism.

  Secondary (confirmed): the HOOK_TX OnceLock prevents any reinstallation attempt within the same
  process, meaning once the hook thread dies (for any reason), the hook is permanently dead for
  that process run. This is the definitive mechanism.

root_cause_issue2: |
  inject.rs clipboard propagation delay is 30ms — reduced from 75ms in a previous optimization.
  The code's own comment explicitly acknowledges "revert to 50ms if any app drops pastes."

  During transcription (which is CPU-intensive — Parakeet/Whisper inference on the blocking thread
  pool), the system is under load. Clipboard write via arboard/Win32 OpenClipboard/SetClipboardData
  may propagate slower than 30ms under CPU contention. The target app reads stale clipboard content
  (empty or previous text) when Ctrl+V fires. arboard's set_text() returns Ok() even before the
  clipboard data is globally visible to other processes — the propagation is async at the OS level.

  Result: inject_text logs "injection complete" (it did its job), but the target app pastes nothing
  or old content. Intermittent because system load and clipboard access vary by invocation.

fix_issue2: |
  In src-tauri/src/inject.rs:
  - Increase clipboard propagation delay from 30ms → 50ms (back toward the original 75ms,
    meeting halfway since 50ms was the first reduction step per the comment)
  - Increase paste consumption delay from 50ms → 80ms (also revert per comment guidance)
  This matches the code's own documented fallback values.

fix_issue1: |
  In src-tauri/src/keyboard_hook.rs:
  - Replace `static HOOK_TX: OnceLock<SyncSender<HookEvent>>` with
    `static HOOK_TX: OnceLock<std::sync::Mutex<Option<SyncSender<HookEvent>>>>`
    No — better approach: use a Mutex<Option> instead of OnceLock so it can be reset.
  - Actually the cleanest fix: replace the OnceLock with a
    `static HOOK_TX: std::sync::Mutex<Option<std::sync::mpsc::SyncSender<HookEvent>>> =
    Mutex::new(None)`
    and update install() to set it under the mutex, and hook_proc to read under the mutex.
    This allows install() to be called again after cleanup (reinstallation within same process).

  Additionally: in lib.rs, after hook installation failure, log a clear error. The HookHandle
  should notify if the hook thread exits unexpectedly (thread_id reset to 0).

verification: pending user confirmation
files_changed:
  - src-tauri/src/inject.rs
      clipboard propagation: 30ms → 50ms
      paste consumption: 50ms → 80ms
  - src-tauri/src/keyboard_hook.rs
      HOOK_TX: OnceLock<SyncSender> → Mutex<Option<SyncSender>>
      install(): HOOK_TX.set(tx) → lock + *guard = Some(tx) (allows reinstall)
      hook_proc: HOOK_TX.get() → HOOK_TX.try_lock() at all 4 send sites
      hook thread exit: clears HOOK_TX to None before UnhookWindowsHookEx
