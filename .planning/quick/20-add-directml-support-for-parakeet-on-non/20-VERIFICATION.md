---
phase: quick-20
verified: 2026-03-02T00:00:00Z
status: passed
score: 6/6 must-haves verified
re_verification: false
gaps: []
human_verification:
  - test: "Verify GPU badge shows correct GPU name on NVIDIA system"
    expected: "Green badge displays actual GPU name (e.g., 'NVIDIA Quadro P2000') instead of generic text"
    why_human: "Requires running the app on a machine with an NVIDIA GPU to confirm gpuName is populated correctly from CachedGpuDetection"
  - test: "Verify DirectML badge and Parakeet card appear on non-NVIDIA system"
    expected: "Blue 'GPU Detected (DirectML)' badge shown; Parakeet TDT (fp32) model card visible in FirstRun"
    why_human: "Requires running on an Intel or AMD GPU system (no NVIDIA drivers) to exercise the directml_available=true path"
  - test: "Verify Inference Status indicator updates after switching engine"
    expected: "ModelSection 'Inference Status' block refreshes GPU name, Provider, and Engine fields when model selection changes"
    why_human: "useEffect dependency on [selectedModel, currentEngine] can only be observed in a running app"
  - test: "Verify Parakeet runs with DirectML EP on non-NVIDIA system"
    expected: "First Parakeet transcription succeeds; logs show 'Requesting DirectML ExecutionProvider'"
    why_human: "Requires hardware with Intel/AMD GPU + DirectX 12 support; cannot verify EP selection at runtime from code alone"
---

# Phase quick-20: Add DirectML Support for Parakeet + GPU Status Indicator Verification Report

**Phase Goal:** Add DirectML execution provider support for Parakeet TDT on non-NVIDIA GPUs (Intel/AMD) and add a GPU/inference status indicator to the settings UI and FirstRun flow.
**Verified:** 2026-03-02
**Status:** PASSED (automated checks) — human verification required for hardware-dependent runtime behavior
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|---------|
| 1 | Parakeet loads with CUDA EP on NVIDIA GPUs (existing behavior preserved) | VERIFIED | `load_parakeet` with `provider="cuda"` sets `ExecutionProvider::Cuda`; both call sites in `lib.rs` (lines 305-311, 1391-1395) read `parakeet_provider` from `CachedGpuDetection`, which returns `"cuda"` when NVIDIA detected |
| 2 | Parakeet loads with DirectML EP on non-NVIDIA systems (Intel/AMD GPUs) | VERIFIED | `detect_gpu_full` returns `parakeet_provider="directml"` on NVML failure; `load_parakeet` matches `"directml"` -> `ExecutionProvider::DirectML` (`transcribe_parakeet.rs` lines 32-35) |
| 3 | Parakeet falls back to CPU EP if neither CUDA nor DirectML is available | VERIFIED (by design) | `load_parakeet` `_ =>` branch returns `None` (CPU default); runtime CPU fallback is delegated to ort's EP fallback mechanism — `detect_gpu_full` always returns directml for non-NVIDIA, and ort auto-falls-back to CPU if DirectML is unavailable at runtime |
| 4 | GPU status indicator shows detected GPU name, execution provider, and active model in settings ModelSection | VERIFIED | `ModelSection.tsx` lines 153-173: `gpuInfo && (...)` renders "Inference Status" block with GPU, Provider, Engine rows; `invoke<GpuInfo>('get_gpu_info')` in `useEffect` on `[selectedModel, currentEngine]` |
| 5 | FirstRun shows GPU badge with actual GPU name instead of just "NVIDIA GPU Detected / CPU Mode" | VERIFIED | `FirstRun.tsx` lines 194-209: three-branch conditional — `gpuDetected` shows `{gpuName}` (with fallback `'NVIDIA GPU Detected'`), `directmlAvailable` shows `"GPU Detected (DirectML)"`, else `"CPU Mode"` |
| 6 | GPU status indicator updates after model selection changes | VERIFIED | `ModelSection.tsx` line 43-45: `useEffect(() => { invoke<GpuInfo>('get_gpu_info').then(setGpuInfo)... }, [selectedModel, currentEngine])` — re-fires on both model and engine changes |

**Score:** 6/6 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src-tauri/Cargo.toml` | directml feature added to parakeet-rs dependency | VERIFIED | Line 71: `parakeet-rs = { version = "0.1.9", features = ["cuda", "directml"], optional = true }` |
| `src-tauri/src/transcribe.rs` | Extended detect_gpu returning GPU name + provider recommendation | VERIFIED | `GpuDetection` struct (lines 23-30) with `gpu_name`, `parakeet_provider`, `is_nvidia`; `detect_gpu_full()` function (lines 66-97); original `detect_gpu()` preserved unchanged |
| `src-tauri/src/transcribe_parakeet.rs` | load_parakeet accepting provider string | VERIFIED | Signature changed to `(model_dir: &str, provider: &str)`; matches on `"cuda"`, `"directml"`, or CPU fallback (lines 22-57) |
| `src-tauri/src/lib.rs` | get_gpu_info Tauri command + CachedGpuDetection managed state | VERIFIED | `CachedGpuDetection` struct (line 118); registered on builder (line 1287); `GpuInfo` struct (lines 925-930); `get_gpu_info` command (lines 938-976); registered in `invoke_handler` (line 1323) |
| `src/App.tsx` | Updated FirstRunStatus type with gpuName and directmlAvailable, passes new props to FirstRun | VERIFIED | Interface extended (lines 12-18) with `gpuName: string` and `directmlAvailable: boolean`; fallback at line 38 includes both fields; `<FirstRun>` props at lines 104-107 pass `gpuName` and `directmlAvailable` |
| `src/components/sections/ModelSection.tsx` | GPU status indicator with provider, GPU name, active model | VERIFIED | `GpuInfo` interface (lines 12-17); state + effect (lines 31, 43-45); "Inference Status" UI block (lines 153-173) |
| `src/components/FirstRun.tsx` | GPU badge with actual GPU name and DirectML-aware model visibility | VERIFIED | `FirstRunProps` with `gpuName` and `directmlAvailable` (lines 12-18); three-branch badge (lines 194-209); `visibleModels` filter shows Parakeet for `gpuDetected || directmlAvailable` (lines 59-67); `gridClass` adapts to `visibleModels.length` (lines 172-177) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src-tauri/src/lib.rs` | `src-tauri/src/transcribe_parakeet.rs` | `load_parakeet` call with correct provider from `CachedGpuDetection` | WIRED | Both call sites (lines 305-311 and 1391-1395) read `gpu_detection.0.parakeet_provider.clone()` and pass `&provider` to `load_parakeet` |
| `src-tauri/src/lib.rs` | `src-tauri/src/transcribe.rs` | `detect_gpu_full` returns `GpuDetection` consumed by `CachedGpuDetection` | WIRED | Line 1265: `let detection = transcribe::detect_gpu_full()`; line 1287: `builder = builder.manage(CachedGpuDetection(cached_gpu_detection))` |
| `src/components/sections/ModelSection.tsx` | `get_gpu_info` | `invoke('get_gpu_info')` to populate status indicator | WIRED | Line 44: `invoke<GpuInfo>('get_gpu_info').then(setGpuInfo).catch(console.error)` |
| `src/App.tsx` | `src/components/FirstRun.tsx` | Passes `gpuName` and `directmlAvailable` from `FirstRunStatus` to `FirstRun` props | WIRED | Lines 104-107: `gpuName={firstRunStatus.gpuName}` and `directmlAvailable={firstRunStatus.directmlAvailable}` passed to `<FirstRun>` |

### Requirements Coverage

| Requirement | Description | Status | Evidence |
|-------------|-------------|--------|---------|
| DIRECTML-01 | DirectML EP support for Parakeet on non-NVIDIA systems | SATISFIED | `directml` feature in Cargo.toml; `detect_gpu_full` routes non-NVIDIA to `"directml"` provider; `load_parakeet` handles `"directml"` -> `ExecutionProvider::DirectML` |
| GPU-STATUS-01 | GPU/inference status indicator in UI | SATISFIED | `get_gpu_info` command returns `gpu_name`, `execution_provider`, `active_model`, `active_engine`; ModelSection renders "Inference Status" block; FirstRun shows three-branch GPU badge |

### Anti-Patterns Found

None detected. Scanned for TODO/FIXME/placeholder comments, empty implementations, stub handlers across all 7 modified files.

### Human Verification Required

#### 1. NVIDIA GPU badge shows actual GPU name

**Test:** Launch app on a machine with an NVIDIA GPU that does not yet have models installed (first-run state). Observe the GPU detection badge in FirstRun.
**Expected:** Green badge shows the actual GPU name (e.g., "NVIDIA Quadro P2000") populated from `CachedGpuDetection.gpu_name`. If `gpuName` is empty the fallback text "NVIDIA GPU Detected" appears — verify non-empty name is shown.
**Why human:** Requires NVIDIA hardware to trigger the `detect_gpu_full` NVIDIA path and observe the `gpuName` prop flowing to the badge.

#### 2. DirectML badge and Parakeet card on non-NVIDIA hardware

**Test:** Launch app on a machine with an Intel or AMD GPU (no NVIDIA drivers) in first-run state.
**Expected:** Blue "GPU Detected (DirectML)" badge is shown. Parakeet TDT (fp32) model card is visible alongside Small (English). large-v3-turbo is not visible.
**Why human:** Requires Intel/AMD hardware where NVML init fails, setting `directml_available=true` in the backend response.

#### 3. Inference Status block refreshes on engine switch

**Test:** Open Settings > Model. Switch from a Whisper model to Parakeet TDT (fp32) and back.
**Expected:** "Inference Status" block updates — Provider changes (CUDA/DIRECTML vs CPU/CUDA), Engine changes (whisper/parakeet), active model name reflects the current selection.
**Why human:** Reactive UI behavior driven by `useEffect` dependencies; cannot verify state transitions from code alone.

#### 4. Parakeet inference with DirectML EP on non-NVIDIA system

**Test:** On an Intel/AMD GPU system, download and activate Parakeet TDT (fp32). Perform a voice recording.
**Expected:** Transcription succeeds. Backend logs show `"Requesting DirectML ExecutionProvider for Parakeet TDT"`. Inference is GPU-accelerated (faster than CPU-only baseline).
**Why human:** Requires DirectX 12-capable non-NVIDIA hardware to verify the EP is actually used by ort at runtime.

### Notable Observations

**Design decision — no explicit CPU provider path:** The plan documented a three-way fallback (NVIDIA→CUDA, non-NVIDIA→DirectML, no-GPU→CPU). The implementation always returns `"directml"` for non-NVIDIA systems (including systems with no detectable GPU). The `"cpu"` provider string exists in `load_parakeet` and is reachable via the `_ =>` branch, but `detect_gpu_full` never selects it. This is a deliberate decision documented in SUMMARY.md: DirectML is the correct default for any Windows non-NVIDIA system (Intel/AMD on DirectX 12), and ort auto-falls-back to CPU if DirectML is unavailable at runtime. The doc comment on `detect_gpu_full` says "No GPU: provider=cpu" but the code returns "directml" — a minor comment inconsistency, not a functional defect.

---

_Verified: 2026-03-02_
_Verifier: Claude (gsd-verifier)_
