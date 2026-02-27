# Pitfalls Research

**Domain:** Local voice-to-text desktop tool (Tauri 2.0 + whisper.cpp + Windows)
**Researched:** 2026-02-27
**Confidence:** HIGH (multiple verified sources; most pitfalls confirmed by official Tauri GitHub issues and whisper.cpp issue tracker)

---

## Critical Pitfalls

### Pitfall 1: Overlay Window Steals Focus from Target Application

**What goes wrong:**
The floating pill overlay window grabs focus from the user's active application (VS Code, Chrome, Word, etc.) when it appears or is interacted with. After transcription, the clipboard paste fires into the pill overlay itself instead of the original text field — the text disappears or goes nowhere.

**Why it happens:**
Tauri 2.0 on Windows defaults all new windows to `focus: true`. Even when `focus: false` is set in `tauri.conf.json`, this config is unreliable — the window still steals focus on startup (confirmed bug: tauri-apps/tauri #11566). WebView2 has its own focus behavior that can override the `focusable: false` setting on Windows (issue #14102 is macOS but same root cause exists on Windows). Any click on the pill overlay area triggers a focus transfer away from the user's text field.

**How to avoid:**
- Set `focus: false` AND `focusable: false` in `tauri.conf.json` for the overlay window (belt-and-suspenders).
- Never show the overlay in response to a window click — use only keyboard hotkey to trigger.
- Set `skip_taskbar: true` and `always_on_top: true` but test that the overlay does not receive keyboard/mouse focus.
- Use Win32 `WS_EX_NOACTIVATE` extended window style via Tauri's `set_window_builder_attributes` in Rust to prevent the window from activating. This is the authoritative fix — CSS and config alone are insufficient on Windows.
- Record the `HWND` of the foreground window immediately before the hotkey fires, verify it hasn't changed before injecting text (abort if it has).

**Warning signs:**
- Text appears in the overlay/settings window rather than the target app after transcription.
- The target app's cursor blinks disappears when the overlay animates in.
- Manual testing: open Notepad, click into it, press hotkey — does cursor remain in Notepad?

**Phase to address:** Phase covering Overlay Window UI (whichever phase builds the floating pill). Must be the first thing verified before any text injection work.

---

### Pitfall 2: whisper.cpp CUDA Build Silently Falls Back to CPU

**What goes wrong:**
The app builds successfully and runs, but `system_info` shows `CUBLAS = 0`. All inference runs on CPU at 2-4 seconds per utterance instead of 300-500ms on GPU. The developer doesn't notice because the app "works" — it's just slow.

**Why it happens:**
Three independent ways CUDA can fail silently:
1. CMake CUDA architecture not specified for Pascal (sm_61). Newer examples show `sm_86` (Ampere) and the build system doesn't error — it just compiles CPU-only.
2. CUDA Toolkit not in PATH or Visual Studio integration not detected — the MSVC+CUDA integration requires manually copying MSBuildExtensions from the CUDA toolkit to VS BuildCustomizations folder if auto-detection fails.
3. `whisper-rs` build.rs doesn't find CUDA Toolkit headers. No error is thrown; it disables CUBLAS.
4. Missing `LIBCLANG_PATH` env var on Windows causes `whisper-rs` bindgen to fall back to CPU-only compilation.

**How to avoid:**
- Build with explicit Pascal architecture: `cmake -B build -DGGML_CUDA=1 -DCMAKE_CUDA_ARCHITECTURES="61"` (P2000 is sm_61).
- After every build that includes CUDA, verify CUDA is active: run a transcription and check that `ggml_cuda_init` log line appears, OR check GPU utilization in Task Manager — it must show nonzero NVIDIA GPU usage during inference.
- Add a Rust integration test that loads the model and asserts `WhisperContext::full_params()` has CUDA enabled, running at model-expected latency.
- Document the exact build command with verified flags in a `BUILDING.md`.

**Warning signs:**
- `system_info` output shows `CUBLAS = 0`.
- Task Manager GPU panel shows 0% NVIDIA GPU during transcription.
- Transcription of a 5-second utterance takes more than 1 second.
- CI builds on a machine without CUDA produce a binary that is then used on a CUDA machine and users wonder why it's slow.

**Phase to address:** Phase covering whisper-rs integration. Verify GPU usage as the acceptance criterion for the transcription phase — latency benchmark must be the gate.

---

### Pitfall 3: Clipboard Paste Race Condition Corrupts or Loses User's Clipboard Contents

**What goes wrong:**
Two failure modes:
1. The app sets the clipboard to the transcribed text, then simulates Ctrl+V before the target application has read the clipboard. The paste fires but the clipboard hasn't propagated — the target app pastes the previous clipboard contents, not the transcription.
2. The app restores the original clipboard contents too quickly, before the target app finishes reading the transcription text from the clipboard. The user's pasted text is the original clipboard content, not the transcription.

**Why it happens:**
Windows clipboard is a shared asynchronous resource. `SetClipboardData` returns before the data is available to other processes. `OpenClipboard`/`GetOpenClipboardWindow` will show another application has the clipboard locked. No built-in synchronization exists. A naive implementation of `set → send Ctrl+V → restore` runs in <5ms which is too fast for most applications to process.

**How to avoid:**
- Use `GetOpenClipboardWindow` to check that no other app holds the clipboard before writing. Retry with backoff if locked.
- Insert a 50-100ms delay between `SetClipboardData` and the `SendInput(Ctrl+V)` call. BridgeVoice and reference projects have validated this timing.
- Insert a separate 100-150ms delay between the Ctrl+V send and clipboard restore.
- Do NOT use `CF_TEXT` — use `CF_UNICODETEXT` to avoid encoding conversion race conditions.
- Make the delay configurable (some slow machines need 200ms).

**Warning signs:**
- Users report occasionally getting their old clipboard content pasted instead of the transcription.
- The issue is intermittent and harder to reproduce on developer machines (which are faster) than on user machines.
- Does not reproduce in isolated test but appears under load (e.g., when browser tabs are loading).

**Phase to address:** Phase covering text injection. The delay must be built in from day one, not retrofitted after user reports.

---

### Pitfall 4: Windows Defender Flags the Binary as Malware (Keyboard Injection)

**What goes wrong:**
The app is flagged by Windows Defender as a HackTool or suspicious binary because it uses `SendInput` for keyboard simulation. Unsigned binaries that perform low-level keyboard/clipboard operations are a common malware pattern. Users can't install or run the app, or Defender quarantines it silently.

**Why it happens:**
`SendInput` and `SetClipboardData` followed by simulated keystrokes are identical at the API level to keylogger/inject behavior. Windows Defender's ML classifier uses heuristics — unsigned binaries that use these APIs are high-risk candidates. The `enigo` crate wraps exactly these APIs. Similar tools (Winaero Tweaker, LibreHardwareMonitor, Keyran) have all been incorrectly flagged as HackTool in 2024-2025 due to Microsoft content definition updates.

**How to avoid:**
- **Code signing is required for distribution.** An OV (Organization Validation) or EV (Extended Validation) code signing certificate from DigiCert, Sectigo, or similar CA will eliminate most false positives. EV certificates provide SmartScreen reputation immediately.
- Without a certificate, submit the binary to Microsoft's Malware Protection Center (MPC) for exclusion — but this only helps for a specific binary hash, not future builds.
- For personal/friend distribution without a cert: document that users must add a Defender exclusion for the app folder, and provide clear install instructions.
- Do NOT use `enigo`'s character-by-character keystroke injection as the primary method — it generates more API calls and is more likely to trigger heuristics. Prefer clipboard paste (fewer SendInput calls).

**Warning signs:**
- Windows SmartScreen blocks the installer on first run ("Unknown publisher" warning).
- Defender quarantines the binary in `%AppData%\Microsoft\Windows Defender\Quarantine`.
- Users report "Windows protected your PC" dialog blocking launch.

**Phase to address:** Installer/distribution phase. Budget for a code signing certificate from the start. Do not assume personal use bypasses this — friends' machines have Defender enabled and cannot add exclusions as easily.

---

### Pitfall 5: Whisper Hallucinations on Silence or Background Noise

**What goes wrong:**
When the user presses the hotkey, pauses before speaking (or accidentally triggers hold-to-talk without speaking), whisper.cpp receives near-silence or ambient noise and generates hallucinated text — fabricated sentences, repeated phrases, or garbage Unicode characters — which get injected into the user's document.

**Why it happens:**
Whisper was trained on speech data. When fed non-speech audio, it tries to "find" speech and generates text anyway. This is a fundamental model behavior, not a bug. The `initial_prompt` parameter makes it worse — a structural engineering prompt biases the decoder toward engineering terms, so hallucinations tend to produce plausible-sounding but wrong engineering content ("the W-section 14x90 should be placed...").

**How to avoid:**
- Use Silero VAD as the gate — only pass audio to whisper.cpp if VAD confirms speech was detected. Do not pass audio buffers where VAD returned zero speech frames.
- Implement a minimum speech duration threshold: if VAD detected less than 300ms of speech total, discard the buffer without calling whisper.cpp.
- After transcription, check for hallucination indicators: output is longer than 200% of the audio duration in word-count terms; output contains repetitive patterns (repeated phrases); output is in a different language than expected. If any indicator triggers, discard rather than inject.
- Log discarded hallucinations for debugging.

**Warning signs:**
- Pressing and releasing the hotkey without speaking injects text.
- Users report occasional random sentences appearing in their documents.
- In toggle mode, a brief press (< 0.5 second) produces injected text.

**Phase to address:** Phase covering VAD integration and the transcription pipeline. The VAD gate must be implemented before any end-to-end testing.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Skip clipboard save/restore | 50ms faster injection | User's clipboard contents are destroyed on every dictation | Never — restore is non-negotiable |
| Hardcode hotkey instead of making configurable | Faster MVP | Users can't change it; conflicts with existing shortcuts are unconfigurable | Only in proof-of-concept phase, not first release |
| Bundle model in installer | Simpler first-run UX | NSIS installer fails at 2GB (confirmed bug #7372); installer size is 1.5-3GB | Never for NSIS — use model download on first run |
| Use `enigo` char-by-char injection only | Simpler code | Slow for long text; Windows Terminal has Unicode bugs; more Defender heuristic triggers | Only as explicit fallback, never as default |
| Skip CUDA architecture flag in build | Build "works" | CPU-only inference, 4-8x slower transcription silently | Never — always specify `sm_61` for P2000 |
| Use `threshold: 0.5` default for VAD | Works for most speech | Too sensitive in noisy environments; too conservative for soft-spoken users | Start here, but expose as user-configurable |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| whisper-rs + CUDA on Windows | Assume `cargo build` auto-detects CUDA | Set `CUDA_PATH` env var, specify `CMAKE_CUDA_ARCHITECTURES="61"` for P2000, verify with GPU usage check |
| cpal + WASAPI 16kHz | Request 16kHz directly from device | Most Windows audio devices default to 44.1kHz or 48kHz. Request device's native rate, then resample to 16kHz using `rubato` or `dasp` crate. Using `AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM` can introduce quality issues |
| Silero VAD via `ort` | Load ONNX model synchronously on main thread | Load model async on startup, keep loaded in memory for session lifetime — re-loading per utterance adds 50-200ms |
| tauri-plugin-store | Store settings as flat key-value | Structure settings as nested JSON from day one — retrofitting structure after shipping requires migration logic |
| whisper.cpp initial_prompt | Set a long prompt with all vocabulary | Long prompts consume tokens from the model's context window, leaving less room for transcription. Keep initial_prompt to 50-100 words max |
| Windows clipboard API | Use Win32 directly from multiple threads | Clipboard must be opened/closed on a single thread. Never call clipboard APIs from the audio thread or tokio task — dispatch to a dedicated clipboard thread |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Loading whisper model on each transcription | 2-5 second model load delay before every utterance | Load model once at startup, keep in memory for the app lifetime | From the very first use |
| Allocating a new audio buffer per utterance | Memory fragmentation after hours of use | Pre-allocate a fixed-size ring buffer at startup | After 1-4 hours of continuous use |
| Running VAD on the main Tauri thread | UI freezes during speech detection | Move audio capture and VAD to a dedicated Rust thread; use channels to communicate with main thread | Immediately on any speech activity |
| Re-reading corrections JSON from disk on every transcription | 5-50ms disk I/O per utterance adds to latency | Load corrections dictionary into memory at startup, watch for file changes with `notify` crate | From first use on HDD systems |
| Large correction dictionary with O(n) scan | Latency grows as user adds corrections | Use `HashMap<String, String>` not `Vec<(String, String)>` for O(1) lookups | At ~100+ correction entries |

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Writing transcription history to disk without user consent | Privacy violation; raw voice transcripts contain sensitive information | Default to no history persistence. If added, make it opt-in, store in `%APPDATA%\VoiceType\` not in documents/desktop |
| Downloading model files over HTTP | Man-in-the-middle can replace model with malicious binary | Always use HTTPS for model downloads. Verify SHA256 checksum of downloaded model before loading |
| Storing settings in plain text in a world-readable location | Not sensitive for this app, but good hygiene | Use `tauri-plugin-store` which writes to `%APPDATA%\VoiceType\` — correct location by default |
| Logging transcribed text to a debug log file | Raw voice transcriptions are sensitive | Never log transcription content. Log only metadata (duration, word count, latency) |

---

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| No visual feedback during model loading | Users think app is broken when first launched | Show a loading state in the tray icon or a one-time onboarding window during model download/initialization |
| Pill overlay blocks text cursor position | User can't see where text will be injected | Position pill at corner of screen (bottom-right) by default, never near the center where cursor likely is |
| No indication that recording has started | Users speak before recording is active; miss the beginning of utterance | Pill must animate/change color immediately (<50ms) when hotkey is pressed — do not wait for first VAD frame |
| Injecting text with trailing newline | Moves cursor to next line after every dictation | Strip trailing whitespace and newlines from whisper.cpp output before injection |
| Toggle mode with no maximum recording time | User accidentally leaves recording running for hours | Implement a maximum recording duration (e.g., 60 seconds) with an audio cue and auto-stop |
| Settings window requires app restart to apply changes | Frustrating developer experience | Apply hotkey and VAD threshold changes at runtime without restart; only model switches require reload |
| Hold-to-talk requires holding during full transcription | User must keep key held while waiting for whisper | Release key ends recording; transcription runs asynchronously; key is available immediately for next use |

---

## "Looks Done But Isn't" Checklist

- [ ] **Clipboard paste:** Verify that user's original clipboard contents are fully restored after paste, not just the transcription text. Test with clipboard content that is an image (not just text).
- [ ] **Focus preservation:** Test overlay window appearance in each target app: VS Code, Chrome address bar, Windows Terminal, Outlook compose window, AutoCAD command line. Verify cursor remains in target app throughout.
- [ ] **GPU utilization:** Confirm Task Manager shows NVIDIA GPU activity during transcription, not just CPU. `CUBLAS = 0` means CPU-only.
- [ ] **Hold-to-talk release:** Test that releasing the hotkey while audio is processing does not block the next hotkey press. The hotkey must be available immediately for the next utterance.
- [ ] **First-run model download:** Verify that the download UI shows progress, handles interruption gracefully (resume or retry), and validates the SHA256 checksum before loading the model.
- [ ] **CPU fallback:** Test the installer on a machine without NVIDIA GPU. Verify that the app detects no CUDA, loads the `small` model, and functions end-to-end at slower speed without crashing.
- [ ] **Corrections dictionary:** Verify that corrections are case-insensitive where expected (e.g., "i beam" and "I Beam" both map to "I-beam"). Test the correction editor UI with 50+ entries.
- [ ] **VAD in toggle mode:** Verify that silence detection actually stops recording and fires transcription. Test in a quiet room (works?) and with ambient noise (does threshold need adjustment?).

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Overlay steals focus (shipped without fix) | HIGH | Requires adding Win32 `WS_EX_NOACTIVATE` to Rust window builder — involves rebuilding Rust backend and full regression test |
| CPU-only binary shipped to users | MEDIUM | Push new build with correct CUDA architecture flags; auto-update if available; manual reinstall otherwise |
| Clipboard race condition complaints | LOW | Increase delay constants in `injector.rs` from 50ms to 100-200ms; ship patch build |
| Defender flagging | HIGH | Purchase OV code signing certificate (~$300-500/year), sign binary, re-release. Short-term: document manual Defender exclusion for users |
| Hallucination injection complaints | MEDIUM | Implement VAD minimum duration gate (if not already present) and hallucination detection heuristics; ship patch |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Overlay steals focus | Overlay window phase | Manual test: focus Notepad, trigger hotkey, verify cursor remains in Notepad |
| CUDA silent fallback | whisper-rs integration phase | Acceptance criterion: Task Manager GPU > 0% during transcription; latency < 500ms |
| Clipboard race condition | Text injection phase | Acceptance criterion: 20 consecutive pastes with clipboard restore verified correct |
| Windows Defender flagging | Distribution/installer phase | Test installer on a fresh Windows 10 VM with default Defender settings |
| Whisper hallucinations | VAD + transcription pipeline phase | Acceptance criterion: pressing hotkey without speaking injects nothing |
| cpal sample rate mismatch | Audio capture phase | Log actual device sample rate; verify resampling produces clean 16kHz audio (playback test WAV) |
| Model load delay per utterance | whisper-rs integration phase | Verify model is loaded once at startup by measuring time-to-first-inference vs. subsequent inferences |
| Focus not configurable by user | Settings panel phase | Test that changing hotkey in settings takes effect without app restart |

---

## Sources

- [Tauri transparent window issue #8308](https://github.com/tauri-apps/tauri/issues/8308) — MEDIUM confidence (GitHub issue, multiple confirmations)
- [Tauri transparent window ghost titlebar #14764](https://github.com/tauri-apps/tauri/issues/14764) — MEDIUM confidence (confirmed bug, open as of early 2026)
- [Tauri focus: false config broken #11566](https://github.com/tauri-apps/tauri/issues/11566) — MEDIUM confidence (confirmed regression)
- [Tauri click-through transparent windows #13070](https://github.com/tauri-apps/tauri/issues/13070) — MEDIUM confidence (feature request + confirmed limitation)
- [Tauri global hotkey system keys not captured #13919](https://github.com/tauri-apps/tauri/issues/13919) — MEDIUM confidence (confirmed Windows-specific behavior)
- [NSIS >2GB installer bug tauri-apps/tauri #7372](https://github.com/tauri-apps/tauri/issues/7372) — HIGH confidence (confirmed hard limit)
- [whisper.cpp CUDA not detected issue #2857](https://github.com/ggml-org/whisper.cpp/issues/2857) — MEDIUM confidence (confirmed CUDA detection issue on Windows)
- [cpal sample rate mismatch issue #593](https://github.com/RustAudio/cpal/issues/593) — HIGH confidence (confirmed WASAPI limitation)
- [cpal resampling noise issue #135](https://github.com/RustAudio/dasp/issues/135) — MEDIUM confidence (confirmed quality issue with naive downsampling)
- [Silero VAD comparison and threshold analysis — Picovoice](https://picovoice.ai/blog/best-voice-activity-detection-vad-2025/) — MEDIUM confidence (vendor benchmark, directionally accurate)
- [Whisper hallucination on empty audio — OpenAI community](https://community.openai.com/t/whisper-api-hallucinating-on-empty-sections/93646) — HIGH confidence (widely reported, reproducible)
- [Windows Defender HackTool false positives — multiple forum reports, 2024-2025](https://forum.juce.com/t/what-to-do-about-windows-defender-false-positives/66438) — MEDIUM confidence (pattern well-documented, enigo specifically LOW confidence)
- [Technical research artifact: 2026-02-27-voice-to-text-desktop-tool-technical.md](artifacts/research/2026-02-27-voice-to-text-desktop-tool-technical.md) — HIGH confidence (pre-validated findings from prior research session)

---
*Pitfalls research for: local voice-to-text desktop tool (Tauri 2.0 + whisper.cpp + Windows)*
*Researched: 2026-02-27*
