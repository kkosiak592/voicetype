# Technical Research: Voice-to-Text Desktop Tool

## Strategic Summary

BridgeVoice validates the exact architecture you should build: **Tauri 2.0 + whisper.cpp (via whisper-rs) + clipboard paste text injection**. Your Quadro P2000 (5GB VRAM, CUDA 11.7) can comfortably run whisper.cpp's large-v3-turbo model with sub-500ms transcription latency. The key tradeoff is between Tauri (smaller binary, native Rust integration, BridgeVoice-proven) and Python+PySide6 (faster prototyping, native faster-whisper access). Tauri is the recommended path given your distribution goals and the existing reference implementations.

---

## Requirements

- **Latency**: Sub-500ms from end-of-speech to text appearing (top priority)
- **Local/Offline**: No internet dependency for core transcription
- **GPU**: NVIDIA Quadro P2000 (5GB VRAM, Pascal arch, CUDA 11.7)
- **CPU Fallback**: Must work on laptops without NVIDIA GPU
- **UI**: Floating pill overlay, system tray, global hotkey activation
- **Dictation Modes**: Both hold-to-talk and toggle on/off
- **Custom Vocabulary**: Domain-specific word corrections for structural engineering
- **Text Injection**: Into any active application
- **Settings**: Extensible settings panel (corrections, shortcuts, formatting)
- **Packaging**: Single installer for Windows distribution

---

## How BridgeVoice Works (Your Primary Reference)

| Aspect | BridgeVoice | Wispr Flow |
|--------|-------------|------------|
| **Processing** | 100% local (whisper.cpp) | 100% cloud (proprietary ASR + Llama LLM) |
| **Framework** | Tauri 2.0 (Rust) | Electron |
| **Latency** | < 500ms (local GPU) | < 700ms p99 (cloud) |
| **Text injection** | Clipboard + paste simulation | Accessibility API |
| **STT engine** | whisper.cpp compiled via Rust FFI | Proprietary cloud ASR |
| **GPU accel** | Metal (macOS), CUDA (Windows) | N/A (cloud) |
| **RAM usage** | Low (Tauri ~20-40MB + model) | ~800MB (Electron) |
| **Transcription mode** | Chunk-based (transcribe after pause) | Streaming (50ms chunks via WebSocket) |
| **LLM cleanup** | None (raw Whisper output) | Fine-tuned Llama for formatting/corrections |
| **Privacy** | Zero telemetry, works offline | Cloud processing, optional screenshot capture |
| **Context awareness** | None | Screenshots of active window (controversial) |

BridgeVoice's architecture is the right model for your project. Key implementation details:
- **Persistent audio stream** starts recording in <10ms
- **whisper-rs** Rust bindings for whisper.cpp integration
- **Clipboard paste** (Ctrl+V simulation) for universal text injection
- **Two recording modes**: Push-to-talk (hold hotkey) and Toggle (press to start/stop)
- **Supported models**: tiny (75MB) through large-v3 (3.1GB) and turbo (~1.5GB)
- Audio visualizer with 7 frequency bands in the floating widget

---

## STT Model Analysis

### Model Comparison Table

| Model/Runtime | Latency (5s utterance) | VRAM | RAM (CPU) | WER | Streaming | Windows CUDA |
|---|---|---|---|---|---|---|
| **whisper.cpp large-v3-turbo** (GPU) | ~300-500ms | ~2.5 GB | N/A | ~2% | No (chunk) | CUDA 11.x OK |
| **whisper.cpp medium** (GPU) | ~500ms-1s | ~2.1 GB | N/A | ~3-4% | Partial | CUDA 11.x OK |
| **whisper.cpp small** (CPU) | ~2-4s | N/A | ~852 MB | ~5-6% | Partial | N/A |
| **faster-whisper turbo int8** (GPU) | ~200-500ms | 1545 MB | N/A | 1.9% | No (chunk) | **CUDA 12 required** |
| **faster-whisper small int8** (CPU) | ~8-10s (13min batch) | N/A | 1477 MB | ~5-6% | No | N/A |
| **Moonshine Medium Streaming** (CPU) | **107-269ms** | N/A | ~500-800 MB | 6.65% | **Yes (native)** | N/A (CPU) |
| **Moonshine Small Streaming** (CPU) | **73-165ms** | N/A | ~300-500 MB | 7.84% | **Yes (native)** | N/A (CPU) |
| **Vosk large** | ~200-600ms | N/A | ~2-4 GB | ~5-8% | Yes (native) | N/A (CPU) |
| **sherpa-onnx Zipformer** | ~100-200ms | N/A | ~200-500 MB | ~8-12% | Yes (native) | ONNX RT |

### Critical Finding: CUDA Version Compatibility

- **whisper.cpp**: Supports CUDA 11.x -- **compatible with your P2000 (CUDA 11.7)**
- **faster-whisper (CTranslate2)**: Latest requires CUDA 12 + cuDNN 9 -- **incompatible without CUDA upgrade**
- This makes **whisper.cpp the clear choice** for your current hardware

### Recommended Model Strategy

**For GPU (P2000 with CUDA 11.7):**
- **whisper.cpp large-v3-turbo** (~2.5GB VRAM) -- best accuracy-to-speed ratio, fits your 5GB VRAM
- Fallback to **medium** (~2.1GB) if turbo has issues on Pascal architecture
- Build with CMake flag `-DGGML_CUDA=1`, requires MSVC + CUDA Toolkit 11.7

**For CPU-only (laptops):**
- **whisper.cpp small** (~852MB RAM) -- usable accuracy, reasonable speed
- Or **Moonshine Small/Medium Streaming** for sub-200ms partial results (future enhancement)

### Quantization Opportunity

whisper.cpp supports GGML quantized models:
- **tiny Q5_1**: 31 MB (vs 75 MB full)
- **small Q5_1**: 182 MB (vs 466 MB full)
- Reduces size by ~45% with same WER and 19% lower latency

### Moonshine (Future Enhancement, Not V1)

Moonshine v2 (Feb 2026) achieves remarkable streaming latency on CPU:
- 69ms on x86 Linux for Tiny Streaming
- 269ms for Medium Streaming (6.65% WER)
- ONNX Runtime based, works on Windows
- C++ core with ONNX deployment
- Could replace whisper.cpp for CPU-only machines if streaming partial results are desired

---

## App Framework Analysis

### Approach 1: Tauri 2.0 (Rust + Web Frontend) -- RECOMMENDED

**How it works:** Rust backend handles audio capture, whisper.cpp inference, global hotkeys, text injection. Web frontend (React/Svelte/Vue) handles UI for pill overlay, settings panel, history.

**Libraries/tools:**
- `tauri` v2.x -- desktop framework
- `whisper-rs` v0.15+ -- Rust bindings to whisper.cpp with CUDA support
- `cpal` -- cross-platform audio capture (used by Keyless, Handy, Voquill)
- `global-hotkey` crate -- system-wide keyboard shortcuts
- `tray-icon` crate -- system tray (built into Tauri)
- `enigo` or Win32 `SendInput` -- keyboard/clipboard text injection
- `silero-vad` via `ort` (ONNX Runtime for Rust) -- voice activity detection
- React or Svelte for frontend UI

**Pros:**
- BridgeVoice validates this exact stack in production
- 3+ open-source voice-to-text reference projects (Keyless, Handy, Voquill, Pothook)
- Tiny installer (~2.5 MB for app, plus model files)
- Low RAM (~20-40 MB for Tauri itself)
- Native Rust FFI to whisper.cpp -- no sidecar, no IPC overhead
- Built-in NSIS/WiX installer generation for Windows
- Uses system WebView2 (included in Windows 10/11)

**Cons:**
- Rust learning curve if unfamiliar
- Transparent window bug in Tauri v2 on Windows (workarounds exist, documented in Issues #8308, #13270)
- NSIS/WiX fail for bundles > 2GB (model files should be downloaded separately, not bundled)

**Best when:** Building for distribution, low resource usage matters, you want the same stack BridgeVoice uses

**Complexity:** M

**Known Issues & Workarounds:**
- Transparent overlay: Set `transparent: true` in window config + CSS `background: transparent`. Multiple projects ship with this working.
- Large model bundling: Download models on first run rather than bundling in installer.

### Approach 2: Python + PySide6

**How it works:** Python app using PySide6 for UI, faster-whisper for inference, sounddevice for audio capture, pynput for hotkeys and text injection.

**Libraries/tools:**
- `PySide6` -- Qt-based UI framework
- `faster-whisper` -- CTranslate2-based Whisper (requires CUDA 12 for GPU, or CPU-only)
- `sounddevice` -- PortAudio bindings for audio capture
- `pynput` -- global hotkeys and keyboard simulation
- `pyperclip` -- clipboard operations
- `silero-vad` -- voice activity detection (built into faster-whisper)
- PyInstaller or Nuitka for packaging

**Pros:**
- Fastest path to working prototype (Python is higher-level)
- faster-whisper is native Python -- no FFI needed
- Silero VAD already integrated into faster-whisper
- Reference projects exist (OmniDictate, Whisper-Writer)
- Qt has excellent overlay window support (FramelessWindowHint, WindowStaysOnTopHint, WA_TranslucentBackground)

**Cons:**
- **CUDA 12 required** for faster-whisper GPU -- incompatible with your P2000's CUDA 11.7 without upgrade
- Bundle size: 500MB-2.2GB with CUDA dependencies (vs 2.5MB for Tauri)
- PyInstaller packaging is fragile and produces large bundles
- pynput global hotkeys have known issues on Windows 11 with transparent overlays
- RAM: 50-100MB baseline for Python + Qt

**Best when:** Rapid prototyping, you're most comfortable in Python, GPU CUDA version isn't a constraint

**Complexity:** S (development) / M (packaging)

### Approach 3: .NET / WPF

**How it works:** Native Windows app using WPF for UI, whisper.cpp via Whisper.net NuGet package, NAudio for audio capture, RegisterHotKey Win32 API for global shortcuts.

**Libraries/tools:**
- WPF (.NET 8) -- native Windows UI
- `Whisper.net` NuGet package -- C# bindings to whisper.cpp with CUDA 11/12/13 runtime packages
- NAudio -- audio capture
- `RegisterHotKey` P/Invoke -- global hotkeys
- `H.NotifyIcon` -- system tray
- `SendInput` P/Invoke -- text injection

**Pros:**
- Most reliable overlay/tray/hotkey behavior on Windows (native platform)
- Whisper.net includes pre-built CUDA runtime NuGet packages (CUDA 11.x supported!)
- Small, efficient packaging (MSIX/MSI, can be single EXE)
- No transparent window bugs (WPF handles this natively)
- Low RAM (~30-60MB)

**Cons:**
- Windows-only (no cross-platform)
- Smaller community for speech-to-text in .NET
- No reference voice-to-text projects in .NET/WPF to follow
- WPF is older (though stable and well-documented)
- Settings UI would need to be built from scratch (no web framework convenience)

**Best when:** Windows-only is acceptable, you want the most rock-solid native Windows experience

**Complexity:** M

### Framework Comparison

| Aspect | Tauri 2.0 | Python+PySide6 | .NET/WPF |
|--------|-----------|----------------|----------|
| Installer size | ~2.5 MB + models | 500MB-2.2GB | ~20 MB + models |
| RAM usage | 20-40 MB | 50-100 MB | 30-60 MB |
| CUDA 11.7 compat | Yes (whisper.cpp) | No (needs CUDA 12) | Yes (Whisper.net) |
| Global hotkeys | Excellent | Good (caveats) | Excellent |
| Floating overlay | Good (workaround needed) | Good | Excellent |
| System tray | Excellent | Excellent | Excellent |
| Whisper integration | Excellent (Rust FFI) | Excellent (native) | Good (NuGet) |
| Reference projects | 3+ voice-to-text apps | 2+ voice-to-text apps | None |
| Dev speed | Medium | Fast | Medium |
| Packaging quality | Excellent | Fair | Excellent |
| Cross-platform | Yes | Yes | No |

---

## Text Injection Analysis

### Method 1: Clipboard Paste (Ctrl+V) -- RECOMMENDED DEFAULT

**How it works:** Copy text to clipboard → simulate Ctrl+V keypress via SendInput

- Used by BridgeVoice, Handy, and most proven tools
- Extremely fast regardless of text length
- Works in most GUI applications
- **Downside**: Temporarily overwrites clipboard (must save and restore)
- **Edge cases**: Doesn't work in some password fields, VMs without shared clipboard, some Remote Desktop scenarios
- Race condition risk: applications may process paste before clipboard is fully set (mitigate with `GetOpenClipboardWindow` check)

### Method 2: SendInput Character-by-Character -- FALLBACK

**How it works:** Each character sent as WM_KEYDOWN/WM_KEYUP with VK_PACKET and Unicode codepoint

- Works in terminals, editors, browsers, password fields
- Handles Unicode natively
- **Downside**: Slow for long text (each char = 2 messages), OS limits ~5000 chars
- Windows Terminal has Unicode bugs (Issue #12977)
- Used by OmniDictate (pynput), Whispering (enigo crate)

### Method 3: UI Automation (ValuePattern.SetValue) -- NOT RECOMMENDED

- Only works if target control implements ValuePattern
- Cannot insert at cursor position -- replaces entire value
- Doesn't work in web browsers, Electron apps, custom controls
- Too limited for general-purpose voice typing

### Recommended Hybrid Strategy

1. **Default**: Clipboard paste (Ctrl+V) -- fast, reliable for 95% of apps
2. **Fallback**: SendInput character-by-character -- for terminals, password fields
3. **Save/restore clipboard** with timing checks
4. **Make configurable** per-app in settings

---

## Audio Capture

### Rust/Tauri (Recommended)

**cpal** -- the standard for Rust audio I/O:
- Cross-platform (WASAPI on Windows, CoreAudio on macOS, ALSA on Linux)
- Callback-based, dedicated audio thread
- Used by Keyless, Handy, Voquill
- Production-ready, well-maintained

Also available:
- `tauri-plugin-mic-recorder` -- cpal + hound, produces WAV files
- `tauri-plugin-audio-recorder` -- alternative Tauri audio plugin

### Python Alternative

**sounddevice** -- PortAudio bindings:
- Callback-based real-time capture
- 1-1.5ms latency achievable with WDM-KS drivers
- Works at 16kHz (Whisper's native sample rate)

### Voice Activity Detection (VAD)

**Silero VAD** is the industry standard:
- 1.8 MB model (MIT licensed)
- Processes 30ms audio chunks in ~1ms on CPU
- Supports 8kHz and 16kHz sample rates
- Available as ONNX model (use `ort` crate in Rust)
- Already integrated into faster-whisper (Python)
- Keyless project includes Rust VAD integration
- Configurable: `threshold`, `min_silence_duration_ms`, `speech_pad_ms`

---

## Custom Word Correction (Domain Vocabulary)

For structural engineering terminology, implement a **post-processing pipeline**:

### Level 1: Simple Find-and-Replace Dictionary
```json
{
  "corrections": {
    "I beam": "I-beam",
    "w section": "W-section",
    "kips": "kips",
    "pascal": "Pascal",
    "rebar": "rebar",
    "pre stressed": "prestressed",
    "post tension": "post-tension"
  }
}
```
User-editable JSON/TOML file. Applied after every transcription.

### Level 2: Regex-Based Corrections
Pattern matching for common misheard terms:
- "why section" → "W-section"
- "eye beam" → "I-beam"
- "mega pascals" → "MPa"

### Level 3: Whisper Initial Prompt (whisper.cpp feature)
whisper.cpp supports an `--initial-prompt` parameter that biases the model toward specific vocabulary:
```
--initial-prompt "structural engineering, I-beam, W-section, rebar, prestressed concrete, kips, PSI, MPa, AISC, ACI 318"
```
This significantly improves recognition of domain-specific terms without fine-tuning.

### Level 4: Hot Words (whisper.cpp feature)
whisper.cpp supports `--hotwords` for biasing specific tokens during decoding. Experimental but promising for domain vocabulary.

---

## Reference Open-Source Projects

| Project | Stack | STT Engine | Text Injection | Notes |
|---------|-------|-----------|----------------|-------|
| **[Keyless](https://github.com/hate/keyless)** | Tauri v2 + React | Candle ML (Rust) | Keyboard + clipboard | Best Tauri reference, includes VAD |
| **[Voquill](https://github.com/josiahsrc/voquill)** | Tauri + React + Zustand | whisper (local/Groq) | Keyboard (Rust) | Most complete architecture |
| **[Handy](https://github.com/cjpais/Handy)** | Tauri + React | whisper-rs + Parakeet | Paste | Mature, well-documented |
| **[Whispering](https://github.com/braden-w/whispering)** | Svelte 5 + Tauri | whisper.cpp / cloud | enigo (keystroke sim) | ~22MB binary, voice-activated mode |
| **[VoiceTypr](https://github.com/moinulmoin/voicetypr)** | Tauri + React | Local Whisper | Native OS integration | GPU accel on Windows |
| **[OmniDictate](https://github.com/gurjar1/OmniDictate)** | PySide6 | faster-whisper | pynput + pywinauto | Best Python reference |
| **[Whisper-Writer](https://github.com/savbell/whisper-writer)** | PyQt5 | faster-whisper | pynput + pyperclip | Simpler Python architecture |
| **[OpenWhispr](https://github.com/OpenWhispr/openwhispr)** | Electron + React | Whisper + Parakeet | Cascading paste methods | Cross-platform |
| **[Pothook](https://github.com/acknak/pothook)** | Tauri | whisper.cpp (Rust) | N/A | Tauri + whisper.cpp example |

---

## Recommendation

### Stack: Tauri 2.0 + whisper.cpp + Silero VAD

This is the recommended approach because:

1. **BridgeVoice validates it** -- same framework, same STT engine, proven in production
2. **CUDA 11.7 compatible** -- whisper.cpp works with your P2000 out of the box (faster-whisper doesn't)
3. **Smallest distribution** -- 2.5 MB installer + downloadable model files
4. **Multiple reference projects** -- Keyless, Handy, Voquill, Whispering, VoiceTypr all use Tauri
5. **Native Rust integration** -- no sidecar processes, no IPC overhead
6. **Cross-platform potential** -- works on Windows, macOS, Linux

### Specific Libraries

| Component | Library | Version/Notes |
|-----------|---------|---------------|
| Framework | `tauri` | v2.x (latest stable) |
| STT Engine | `whisper-rs` | v0.15+ (Rust bindings to whisper.cpp) |
| Audio Capture | `cpal` | Latest (WASAPI on Windows) |
| VAD | Silero VAD via `ort` | ONNX Runtime for Rust |
| Global Hotkeys | `tauri-plugin-global-shortcut` | Built into Tauri v2 |
| System Tray | `tauri-plugin-tray-icon` | Built into Tauri v2 |
| Text Injection | `enigo` + Win32 clipboard API | Hybrid clipboard paste + keystroke fallback |
| Frontend | React + Tailwind CSS | Or Svelte -- either works |
| Settings Storage | `tauri-plugin-store` | JSON-based persistent storage |
| Model | whisper.cpp large-v3-turbo (GPU) / small (CPU) | GGML format, ~1.5GB / ~466MB |

### Architecture Flow

```
[Global Hotkey Press]
        |
[Start Audio Capture (cpal, 16kHz)]
        |
[Silero VAD monitors for speech]
        |
[User speaks... pill overlay shows recording state]
        |
[Hotkey Release (hold-to-talk) OR VAD silence detected (toggle mode)]
        |
[Audio buffer sent to whisper.cpp via whisper-rs]
        |
[GPU inference: large-v3-turbo (~300-500ms)]
  OR [CPU inference: small model (~2-4s)]
        |
[Post-processing: custom word corrections applied]
        |
[Text injected via clipboard paste (Ctrl+V)]
        |
[Clipboard restored to previous contents]
```

---

## Implementation Context

<claude_context>
<chosen_approach>
- name: Tauri 2.0 + whisper.cpp + Silero VAD
- libraries: tauri v2.x, whisper-rs v0.15+, cpal, ort (ONNX Runtime), enigo, tauri-plugin-global-shortcut, tauri-plugin-store, React, Tailwind CSS
- install: cargo install create-tauri-app, npm create tauri-app@latest
</chosen_approach>
<architecture>
- pattern: Rust backend (audio, inference, injection) + Web frontend (UI, settings)
- components:
  1. Audio Engine (cpal) -- captures mic input at 16kHz
  2. VAD Module (Silero via ort) -- detects speech start/end
  3. Transcription Engine (whisper-rs) -- runs whisper.cpp inference
  4. Post-Processor -- applies custom word corrections
  5. Text Injector (enigo + clipboard) -- injects text into active app
  6. Hotkey Manager (tauri global-shortcut) -- captures system-wide shortcuts
  7. Overlay Window -- floating pill UI (always-on-top, transparent, frameless)
  8. Settings Manager (tauri-plugin-store) -- persists user configuration
  9. System Tray -- background presence with context menu
- data_flow: Hotkey -> Audio Capture -> VAD -> Whisper Inference -> Post-Process -> Text Injection
</architecture>
<files>
- create:
  - src-tauri/src/main.rs -- Tauri app entry, command registrations
  - src-tauri/src/audio.rs -- cpal audio capture, ring buffer
  - src-tauri/src/transcribe.rs -- whisper-rs inference, model management
  - src-tauri/src/vad.rs -- Silero VAD via ONNX Runtime
  - src-tauri/src/injector.rs -- clipboard paste + keystroke text injection
  - src-tauri/src/corrections.rs -- custom word correction post-processing
  - src-tauri/src/hotkeys.rs -- global shortcut management
  - src/App.tsx -- React root
  - src/components/Pill.tsx -- floating recording indicator
  - src/components/Settings.tsx -- settings panel
  - src/components/CorrectionEditor.tsx -- custom word correction UI
  - src/stores/settings.ts -- settings state management
- structure:
  - src-tauri/ -- Rust backend
  - src/ -- React frontend
  - models/ -- downloaded whisper models (gitignored)
  - corrections/ -- user correction dictionaries
- reference: Keyless (https://github.com/hate/keyless), Voquill (https://github.com/josiahsrc/voquill)
</files>
<implementation>
- start_with: Minimal Tauri app with global hotkey that prints to console
- order:
  1. Scaffold Tauri 2.0 + React project
  2. Implement global hotkey capture (hold-to-talk)
  3. Add audio capture with cpal (record to buffer)
  4. Integrate whisper-rs (transcribe buffer, print result)
  5. Add clipboard paste text injection
  6. Build floating pill overlay UI
  7. Add Silero VAD for toggle mode
  8. Build settings panel (model selection, hotkey config)
  9. Add custom word correction system
  10. Add system tray with context menu
  11. Implement model download/management
  12. Package with NSIS installer
- gotchas:
  - Tauri v2 transparent windows on Windows: use CSS background workaround (Issue #13270)
  - whisper-rs CUDA build requires MSVC + CUDA Toolkit 11.7 installed
  - cpal WASAPI may need specific sample rate handling (resample to 16kHz for Whisper)
  - Clipboard restore has race conditions -- add 50-100ms delay before restoring
  - Model files are 500MB-3GB -- download on first run, don't bundle in installer
  - Windows Defender may flag keyboard injection (enigo) -- may need signing
  - NSIS installer fails for bundles > 2GB -- keep models as separate downloads
- testing:
  - Audio capture: record and playback test WAV
  - Transcription: benchmark latency with known audio samples
  - Text injection: test in Notepad, Chrome, VS Code, terminal
  - Hotkeys: verify they work when other apps are focused
  - Overlay: verify transparency and click-through on Windows 10/11
</implementation>
</claude_context>

---

**Next Action:** Scaffold the Tauri 2.0 + React project and implement step 1 (global hotkey that prints to console), or dive deeper into any specific area of this research.

---

## Sources

### BridgeVoice & Wispr Flow
- [BridgeVoice Documentation](https://docs.bridgemind.ai/docs/bridgevoice)
- [BridgeVoice Product Page](https://www.bridgemind.ai/products/bridgevoice)
- [Wispr Flow + Baseten Technical Details](https://www.baseten.co/resources/customers/wispr-flow/)
- [Technical Challenges Behind Flow](https://wisprflow.ai/post/technical-challenges)
- [Wispr Flow WebSocket API](https://api-docs.wisprflow.ai/websocket_quickstart)
- [Wispr Flow Privacy Review (Letterly)](https://letterly.app/blog/wispr-flow-review/)

### STT Models & Benchmarks
- [whisper.cpp GitHub](https://github.com/ggml-org/whisper.cpp)
- [faster-whisper GitHub](https://github.com/SYSTRAN/faster-whisper)
- [faster-whisper turbo v3 benchmark (Issue #1030)](https://github.com/SYSTRAN/faster-whisper/issues/1030)
- [Moonshine GitHub](https://github.com/moonshine-ai/moonshine)
- [Moonshine v2 paper (arXiv)](https://arxiv.org/abs/2602.12241)
- [sherpa-onnx GitHub](https://github.com/k2-fsa/sherpa-onnx)
- [Vosk API GitHub](https://github.com/alphacep/vosk-api)
- [Whisper.net NuGet](https://github.com/sandrohanea/whisper.net)
- [whisper-rs Rust crate](https://crates.io/crates/whisper-rs)
- [Northflank STT Benchmarks 2026](https://northflank.com/blog/best-open-source-speech-to-text-stt-model-in-2026-benchmarks)
- [Tom's Hardware Whisper GPU Benchmarks](https://www.tomshardware.com/news/whisper-audio-transcription-gpus-benchmarked)
- [Choosing Whisper Variants (Modal)](https://modal.com/blog/choosing-whisper-variants)

### Frameworks & Text Injection
- [Tauri v2 Documentation](https://v2.tauri.app/)
- [Tauri transparent window issue #8308](https://github.com/tauri-apps/tauri/issues/8308)
- [Tauri transparent window workaround #13270](https://github.com/tauri-apps/tauri/issues/13270)
- [Tauri vs Electron comparison (Levminer)](https://www.levminer.com/blog/tauri-vs-electron)
- [AutoHotkey SendKeys documentation](https://www.autohotkey.com/docs/v2/howto/SendKeys.htm)
- [Raymond Chen on keyboard simulation](https://devblogs.microsoft.com/oldnewthing/20250319-00/?p=110979)

### Reference Projects
- [Keyless (Tauri v2 + Candle ML)](https://github.com/hate/keyless)
- [Voquill (Tauri + React)](https://github.com/josiahsrc/voquill)
- [Handy (Tauri + whisper-rs)](https://github.com/cjpais/Handy)
- [Whispering (Svelte + Tauri)](https://github.com/braden-w/whispering)
- [VoiceTypr (Tauri + React)](https://github.com/moinulmoin/voicetypr)
- [OmniDictate (PySide6 + faster-whisper)](https://github.com/gurjar1/OmniDictate)
- [Whisper-Writer (PyQt5 + faster-whisper)](https://github.com/savbell/whisper-writer)
- [OpenWhispr (Electron + React)](https://github.com/OpenWhispr/openwhispr)
- [Pothook (Tauri + whisper.cpp)](https://github.com/acknak/pothook)
