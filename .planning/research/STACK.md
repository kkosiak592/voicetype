# Stack Research

**Domain:** Local voice-to-text desktop tool (Windows, GPU-accelerated, offline)
**Researched:** 2026-02-27
**Confidence:** MEDIUM-HIGH (core stack HIGH, some supporting lib versions MEDIUM)

---

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| Tauri | 2.10.2 | Desktop application framework | Smallest binary (~2.5 MB installer), native Rust FFI to whisper.cpp, WebView2-based frontend, BridgeVoice uses this exact stack in production. 20-40 MB RAM vs Electron's 800 MB. |
| whisper-rs | 0.15.1 | Rust bindings to whisper.cpp (STT engine) | Only viable GPU option for CUDA 11.7. faster-whisper mandates CUDA 12. whisper-rs 0.15.1 is the latest stable release (2025-09-10), wraps whisper.cpp with safe Rust FFI. |
| whisper.cpp (via whisper-rs-sys) | 0.14.1 (bundled) | C/C++ inference engine for Whisper models | Supports CUDA 11.x on Pascal architecture (sm_61). Requires building with `-DGGML_CUDA=1 -DCMAKE_CUDA_ARCHITECTURES=61`. Supports GGML quantized models. |
| React | 18.x | Frontend UI framework | Official Tauri template supports React + Vite natively. Voquill and VoiceTypr reference projects use React. Widely known, good TypeScript support, large component ecosystem. |
| Vite | 5.x (via create-tauri-app) | Frontend build tool | Official Tauri recommended bundler for React. Dev server on localhost:5173 that Tauri webview consumes. Fast HMR. |
| TypeScript | 5.x | Frontend language | Strongly recommended by Tauri docs. Type safety for Tauri invoke calls reduces runtime errors in the Rust/JS boundary. |

### Supporting Libraries (Rust)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| cpal | 0.16.0 | Cross-platform audio capture (WASAPI on Windows) | Always — the standard for Rust audio I/O. Callback-based, dedicated audio thread. Used by Keyless, Handy, Voquill. Captures mic at 16kHz for Whisper. |
| silero-vad-rust | latest (~0.1.x) | Voice Activity Detection — bundled ONNX model | For toggle mode (tap to start, auto-stop on silence). Ships Silero ONNX model (opset 15/16) inside the crate — no separate download needed. Uses `ort` internally. |
| ort | 2.0.0-rc.11 | ONNX Runtime for Rust (VAD inference) | Required by silero-vad-rust. Wraps Microsoft's ONNX Runtime 1.23. Note: still RC, but production-used. Alternative: use silero-vad-rust which bundles ort internally. |
| enigo | 0.5.0 | Cross-platform keyboard/mouse input simulation | Text injection via Ctrl+V simulation. Falls back to character-by-character SendInput. Windows-native backend. |
| tauri-plugin-global-shortcut | 2.3.1 | System-wide hotkey registration | Global hotkey capture when Tauri window is not focused. Official Tauri plugin. |
| tauri-plugin-store | 2.4.2 | JSON-based persistent settings storage | User preferences (hotkey config, active profile, corrections). Official Tauri plugin. |

### Supporting Libraries (Frontend)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| Tailwind CSS | 4.2.1 | Utility-first CSS styling | UI styling for pill overlay and settings panel. v4 ships a first-party Vite plugin (`@tailwindcss/vite`) — no `tailwind.config.js` needed, configured in CSS. |
| Zustand | 4.x | Lightweight frontend state management | Settings state, recording state, profile state. Simpler than Redux for this use case. Community Tauri plugin (tauri-plugin-zustand) for backend sync if needed. |

### Models

| Model | File Size | VRAM (estimated) | Use Case |
|-------|-----------|-------------------|----------|
| ggml-large-v3-turbo.bin | 1.5 GiB | ~3-4 GB | Primary: GPU machines (P2000 has 5 GB VRAM — fits comfortably) |
| ggml-large-v3-turbo-q5_0.bin | 547 MiB | ~1.5-2 GB | Quantized alternative: same model, 45% smaller, ~19% faster, same WER |
| ggml-small.bin | 466 MiB | N/A (CPU) | CPU fallback for non-NVIDIA machines |
| ggml-small-q5_1.bin | 182 MiB | N/A (CPU) | Quantized CPU fallback — smaller download, same quality |

**Recommendation:** Ship large-v3-turbo-q5_0 as the GPU default (547 MiB vs 1.5 GiB, fits in 5 GB VRAM, nearly identical accuracy). Use ggml-small-q5_1 as CPU fallback.

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| Rust toolchain (stable) | Build Tauri backend | Required. Install via `rustup`. Minimum 1.77.2 for Tauri plugins. |
| CUDA Toolkit 11.7 | Build whisper.cpp with GPU support | Already installed on dev machine. Required at build time. Set `CUDA_PATH` env var. |
| MSVC (Visual Studio Build Tools) | Compile whisper.cpp on Windows | Required by whisper-rs. Must include C++ and Clang components. Set `LIBCLANG_PATH`. |
| CMake | whisper.cpp build system | whisper-rs-sys uses CMake internally to build whisper.cpp. |
| Node.js 18+ | Frontend toolchain, Tauri CLI | Required for Vite and npm scripts. |
| create-tauri-app | Project scaffolding | `npm create tauri-app@latest` — select React + TypeScript template. |

---

## Installation

```bash
# 1. Create project (select React + TypeScript when prompted)
npm create tauri-app@latest voicetype

# 2. Rust dependencies — add to src-tauri/Cargo.toml
# [dependencies]
# whisper-rs = { version = "0.15", features = ["cuda"] }
# cpal = "0.16"
# silero-vad-rust = "0.1"
# enigo = "0.5"
# tauri-plugin-global-shortcut = "2"
# tauri-plugin-store = "2"

# 3. Add Tauri plugins
pnpm tauri add global-shortcut
pnpm tauri add store

# 4. Frontend dependencies
npm install zustand
npm install tailwindcss @tailwindcss/vite

# 5. Verify CUDA build environment
# Set LIBCLANG_PATH to Visual Studio Llvm bin dir
# Set CUDA_PATH to CUDA Toolkit 11.7 install dir
# Then build with:
cargo build --features cuda
```

**CUDA build flags** — set in `.cargo/config.toml` or environment:
```toml
[env]
GGML_CUDA = "1"
CMAKE_CUDA_ARCHITECTURES = "61"  # Pascal = sm_61
```

---

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| whisper-rs (whisper.cpp) | faster-whisper (CTranslate2) | Only if you upgrade to CUDA 12. faster-whisper is ~30% faster and uses less VRAM, but requires CUDA 12 + cuDNN 9. Not an option on the current P2000 setup. |
| Tauri 2.0 | Electron | Never for this use case. Electron is 800 MB RAM, ~200 MB installer. Tauri is 20-40 MB RAM, 2.5 MB installer. BridgeVoice already validated Tauri works. |
| Tauri 2.0 | Python + PySide6 | Only for rapid prototyping, not distribution. Python bundles are 500 MB-2.2 GB. CUDA 12 still required for GPU in Python path. |
| Tauri 2.0 | .NET/WPF | If Windows-only is a permanent constraint AND you want rock-solid native overlay behavior. Whisper.net (NuGet) does support CUDA 11.x. No cross-platform potential and no reference apps. |
| silero-vad-rust | ort + manual Silero ONNX | silero-vad-rust bundles the model — less setup. Use raw ort only if you need custom VAD model variants. |
| Clipboard paste (enigo) | SendInput char-by-char | Clipboard paste is default (fast, universal). Use SendInput only as fallback for password fields and terminals where Ctrl+V is disabled. |
| large-v3-turbo-q5_0 | large-v3-turbo (f16) | Use q5_0 by default — 45% smaller, 19% faster, same accuracy. Only use f16 if you observe quality degradation. |
| Tailwind CSS v4 | Tailwind CSS v3 | Tailwind v4 has breaking config changes (no tailwind.config.js, CSS-based config). Use v3 only if you need a component library that hasn't updated to v4 yet. |

---

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| faster-whisper / CTranslate2 | Requires CUDA 12 + cuDNN 9. The P2000 with CUDA 11.7 is incompatible. The error is a hard runtime failure, not a degraded experience. | whisper-rs (whisper.cpp) with CUDA 11.7 feature |
| Electron | 800 MB RAM baseline, 200+ MB installer. Kills the lightweight desktop tool goal. OpenWhispr uses Electron and it shows. | Tauri 2.0 |
| tauri-plugin-mic-recorder | High-level wrapper that produces WAV files. Too abstracted for real-time streaming with VAD. Adds unnecessary latency. | cpal directly for ring buffer control |
| UI Automation / ValuePattern text injection | Only works on controls that implement ValuePattern. Fails in browsers, Electron apps, custom controls — i.e., most of the target apps. | Clipboard paste via enigo |
| Bundling models in the installer | NSIS/WiX fail for bundles over 2 GB. The large-v3-turbo binary alone is 1.5 GiB. Model bundling is incompatible with the 2.5 MB installer goal. | Download models on first run; store in app data dir |
| Streaming transcription (Moonshine) | Significantly more complex state machine. Moonshine's Rust integration is immature. v1 latency target (sub-500ms chunk-based) is achievable without streaming. | Chunk-based transcription via whisper-rs |

---

## Stack Patterns by Variant

**If GPU is available (NVIDIA, CUDA 11.7+):**
- Load `ggml-large-v3-turbo-q5_0.bin`
- Build whisper-rs with `features = ["cuda"]`
- CMake: `-DGGML_CUDA=1 -DCMAKE_CUDA_ARCHITECTURES=61`
- Expected latency: 300-500ms for a 5-second utterance

**If CPU-only (no NVIDIA GPU):**
- Load `ggml-small-q5_1.bin`
- Build whisper-rs without cuda feature
- Expected latency: 2-4 seconds (acceptable for toggle mode)
- Do NOT attempt large-v3-turbo on CPU — ~30-60 seconds latency

**If Pascal GPU specifically (sm_61, CUDA 11.7):**
- Use `float32` compute type, not `float16` — Pascal has poor FP16 throughput
- whisper.cpp uses float32 by default in GGML format, so this is handled automatically
- Set `CMAKE_CUDA_ARCHITECTURES=61` explicitly, otherwise the build defaults to a newer target

---

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| whisper-rs 0.15.1 | whisper-rs-sys 0.14.1 | whisper-rs-sys is the C FFI layer; automatically pulled as dependency |
| whisper-rs 0.15.x | CUDA Toolkit 11.7 | Confirmed via feature = "cuda". No CUDA 12 requirement in whisper.cpp. |
| tauri 2.10.2 | tauri-plugin-global-shortcut 2.3.1 | Both on the 2.x line. Plugins track core major version. |
| tauri 2.10.2 | tauri-plugin-store 2.4.2 | Same as above — 2.x line compatibility. |
| Tailwind CSS 4.2.1 | Vite 5.x via @tailwindcss/vite | First-party Vite plugin replaces postcss setup. No tailwind.config.js. |
| ort 2.0.0-rc.11 | ONNX Runtime 1.23 | ort is still RC but production-deployed. silero-vad-rust handles this internally so you may not need to pin ort directly. |
| cpal 0.16.0 | WASAPI (Windows 10+) | Default audio backend on Windows. Resample captured audio to 16kHz before sending to whisper.cpp (Whisper's native sample rate). |

---

## Sources

- **whisper-rs 0.15.1** — [crates.io](https://crates.io/crates/whisper-rs), [docs.rs features](https://docs.rs/crate/whisper-rs/latest/features), [BUILDING.md](https://github.com/tazz4843/whisper-rs/blob/master/BUILDING.md) — HIGH confidence
- **Tauri 2.10.2** — [v2.tauri.app/release/](https://v2.tauri.app/release/), [create-project docs](https://v2.tauri.app/start/create-project/) — HIGH confidence
- **tauri-plugin-store 2.4.2** — [docs.rs](https://docs.rs/crate/tauri-plugin-store/latest) — HIGH confidence
- **tauri-plugin-global-shortcut 2.3.1** — [docs.rs](https://docs.rs/crate/tauri-plugin-global-shortcut/latest) — HIGH confidence
- **Tailwind CSS 4.2.1** — [tailwindcss.com/blog/tailwindcss-v4](https://tailwindcss.com/blog/tailwindcss-v4), GitHub releases — HIGH confidence
- **enigo 0.5.0** — [docs.rs](https://docs.rs/enigo/latest/enigo/), [GitHub](https://github.com/enigo-rs/enigo) — HIGH confidence
- **cpal 0.16.0** — [GitHub RustAudio/cpal](https://github.com/RustAudio/cpal) — HIGH confidence
- **ort 2.0.0-rc.11** — [docs.rs](https://docs.rs/crate/ort/latest) — MEDIUM confidence (still RC)
- **silero-vad-rust** — [crates.io](https://crates.io/crates/silero-vad-rust) — MEDIUM confidence (version not pinned, confirm on crates.io)
- **whisper.cpp CUDA 11.7/Pascal** — [HuggingFace whisper-large-v3-turbo](https://huggingface.co/openai/whisper-large-v3-turbo), [whisper.cpp models README](https://github.com/ggml-org/whisper.cpp/blob/master/models/README.md), CUDA Pascal compatibility confirmed from prior research — MEDIUM confidence (no explicit CUDA 11.7 docs, based on "CUDA 11.x OK" from reference research and community reports)
- **BridgeVoice stack validation** — existing research artifact `artifacts/research/2026-02-27-voice-to-text-desktop-tool-technical.md` — HIGH confidence (production app using same stack)

---

*Stack research for: VoiceType — local voice-to-text desktop tool*
*Researched: 2026-02-27*
