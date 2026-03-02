---
phase: quick-25
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/Cargo.toml
  - src-tauri/src/transcribe.rs
  - src-tauri/src/lib.rs
  - src/components/FirstRun.tsx
autonomous: true
requirements: [QUICK-25]
must_haves:
  truths:
    - "NVIDIA GPU systems recommend Parakeet and show directml_available=false"
    - "Discrete non-NVIDIA GPU systems (AMD/Intel Arc with dedicated VRAM) recommend Parakeet via DirectML"
    - "Integrated-only GPU systems (Intel UHD, AMD APU) recommend small-en and show directml_available=false"
    - "FirstRun UI shows a prominent 'Download Recommended' button that triggers download without requiring card selection"
  artifacts:
    - path: "src-tauri/src/transcribe.rs"
      provides: "has_discrete_gpu() DXGI detection function and updated GpuDetection struct"
      contains: "has_discrete_gpu"
    - path: "src-tauri/src/lib.rs"
      provides: "Updated check_first_run with correct directml_available and recommendation logic"
      contains: "has_discrete_gpu"
    - path: "src/components/FirstRun.tsx"
      provides: "Download Recommended button above model cards"
      contains: "Download Recommended"
  key_links:
    - from: "src-tauri/src/transcribe.rs"
      to: "src-tauri/src/lib.rs"
      via: "GpuDetection.has_discrete_gpu field consumed by check_first_run"
      pattern: "has_discrete_gpu"
    - from: "src-tauri/src/lib.rs"
      to: "src/components/FirstRun.tsx"
      via: "FirstRunStatus.recommendedModel prop drives button label and action"
      pattern: "recommendedModel"
---

<objective>
Add DXGI-based discrete GPU detection so the FirstRun model recommendation correctly distinguishes discrete GPUs (which can run Parakeet via DirectML) from integrated-only GPUs (which should use small-en). Also add a prominent "Download Recommended" button to the FirstRun UI so users can one-click download without scanning model cards.

Purpose: Currently, any non-NVIDIA system blindly sets directml_available=true and shows Parakeet as available, even on integrated-only Intel UHD GPUs where DirectML Parakeet would be painfully slow. This fix ensures the recommendation actually matches hardware capability.

Output: Updated transcribe.rs (DXGI detection), lib.rs (recommendation logic), FirstRun.tsx (download button), Cargo.toml (windows crate dependency).
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src-tauri/src/transcribe.rs
@src-tauri/src/lib.rs
@src-tauri/Cargo.toml
@src/components/FirstRun.tsx

<interfaces>
<!-- From src-tauri/src/transcribe.rs — current GpuDetection struct (will be modified): -->
```rust
pub struct GpuDetection {
    pub gpu_name: String,
    pub parakeet_provider: String,  // "cuda", "directml", or "cpu"
    pub is_nvidia: bool,
}
```

<!-- From src-tauri/src/lib.rs — FirstRunStatus (will be modified): -->
```rust
struct FirstRunStatus {
    needs_setup: bool,
    gpu_detected: bool,
    gpu_name: String,
    directml_available: bool,
    recommended_model: String,
}
```

<!-- From src-tauri/src/lib.rs — CachedGpuDetection wrapper: -->
```rust
pub struct CachedGpuDetection(pub transcribe::GpuDetection);
```

<!-- From src/components/FirstRun.tsx — props interface: -->
```typescript
interface FirstRunProps {
  gpuDetected: boolean;
  gpuName: string;
  directmlAvailable: boolean;
  recommendedModel: string;
  onComplete: (downloadedModelId: string) => void;
}
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add DXGI discrete GPU detection and fix recommendation logic</name>
  <files>src-tauri/Cargo.toml, src-tauri/src/transcribe.rs, src-tauri/src/lib.rs</files>
  <action>
**Cargo.toml** -- Add the `windows` crate as a Windows-only dependency:
```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = ["Win32_Graphics_Dxgi"] }
```
Place this after the existing `[target.'cfg(not(...))'.dependencies]` block (around line 73).

**src-tauri/src/transcribe.rs** -- Three changes:

1. Add `has_discrete_gpu: bool` field to `GpuDetection` struct (after `is_nvidia`).

2. Add a new function `has_discrete_gpu()` that uses DXGI to detect discrete GPUs. Implementation:
   - Use `windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1, IDXGIAdapter1, DXGI_ADAPTER_DESC1, DXGI_ADAPTER_FLAG_SOFTWARE}`.
   - Call `CreateDXGIFactory1::<IDXGIFactory1>()` to get a factory.
   - Enumerate adapters with `factory.EnumAdapters1(i)` in a loop until it returns Err.
   - For each adapter, call `adapter.GetDesc1()` to get `DXGI_ADAPTER_DESC1`.
   - Skip software adapters: if `desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0`, skip.
   - Check `desc.DedicatedVideoMemory > 512 * 1024 * 1024` (512 MB threshold). If any non-software adapter exceeds this, return `true`.
   - Also log each adapter found: name (from `desc.Description` -- convert wide string), dedicated VRAM in MB, flags.
   - If no adapter exceeds the threshold, return `false`.
   - Wrap the entire function body in `#[cfg(target_os = "windows")]` with a fallback that returns `false` on non-Windows.
   - On any DXGI error (factory creation, etc.), log the error and return `false` (safe fallback -- recommends small-en).

3. Update `detect_gpu_full()`:
   - For the NVIDIA path (NVML success): set `has_discrete_gpu: true` (NVIDIA GPUs are always discrete).
   - For the non-NVIDIA paths (both Err branches): call `has_discrete_gpu()` and store the result. Also update `parakeet_provider`: if `has_discrete_gpu()` is true, keep `"directml"`. If false, set `"cpu"`. Update `gpu_name` similarly: if discrete found, keep `"DirectML (auto-detected)"`. If no discrete GPU, set `"Integrated GPU"`.

**src-tauri/src/lib.rs** -- Update `check_first_run()` (around line 1019):

1. Change `directml_available` from `!gpu_mode` to: `detection.0.has_discrete_gpu && !detection.0.is_nvidia`. This means DirectML is only advertised when there is a discrete non-NVIDIA GPU.

2. Change `recommended_model` logic:
   - If `gpu_mode` (NVIDIA): `"parakeet-tdt-v2-fp32"` (unchanged)
   - Else if `detection.0.has_discrete_gpu`: `"parakeet-tdt-v2-fp32"` (discrete AMD/Intel Arc can run Parakeet via DirectML)
   - Else: `"small-en"` (integrated-only or no GPU)

Do NOT change any other consumers of `GpuDetection` or `CachedGpuDetection` -- they access `.gpu_name`, `.parakeet_provider`, `.is_nvidia` which remain valid. The new `has_discrete_gpu` field is only consumed by `check_first_run()`.
  </action>
  <verify>
    <automated>cd src-tauri && cargo check 2>&1 | tail -5</automated>
  </verify>
  <done>
    - GpuDetection struct has `has_discrete_gpu: bool` field
    - `has_discrete_gpu()` function enumerates DXGI adapters and checks DedicatedVideoMemory > 512MB
    - `detect_gpu_full()` populates the new field (true for NVIDIA, DXGI result for others)
    - `check_first_run()` sets `directml_available` only when discrete non-NVIDIA GPU exists
    - `check_first_run()` recommends Parakeet for any discrete GPU, small-en for integrated/no GPU
    - `cargo check` passes with no errors
  </done>
</task>

<task type="auto">
  <name>Task 2: Add "Download Recommended" button to FirstRun UI</name>
  <files>src/components/FirstRun.tsx</files>
  <action>
Add a prominent "Download Recommended" button between the GPU detection badge section (line ~210) and the model cards grid (line ~213).

The button section should:
1. Look up the recommended model object: `const recModel = MODELS.find(m => m.id === recommendedModel)`.
2. Only render if `recModel` exists and `downloadState === 'idle'` (hide during/after download).
3. Render a centered container with:
   - A large button styled with indigo background (`bg-indigo-600 hover:bg-indigo-700 text-white`), rounded-xl, generous padding (`px-8 py-3`), font-semibold, text-base.
   - Button text: `Download Recommended` (just two words, clean).
   - Below the button text (inside the button), a smaller line: `{recModel.name} -- {recModel.size}` in `text-xs opacity-80`.
   - `onClick` calls `handleDownload(recommendedModel)`.
   - `disabled` when `downloadState !== 'idle'`.
4. Add a small divider or "or choose manually" text below the button, before the cards grid. Use `text-xs text-gray-400 dark:text-gray-500 text-center` styling. Something like: "or choose a different model below".
5. Wrap the button + divider in a `<div className="mb-6 text-center">`.

Do NOT change the existing model cards, download logic, or any other behavior. The cards still work as before for power users who want to pick a different model.
  </action>
  <verify>
    <automated>cd "C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text" && npx tsc --noEmit 2>&1 | tail -5</automated>
  </verify>
  <done>
    - "Download Recommended" button appears above the model cards grid on FirstRun screen
    - Button shows recommended model name and size
    - Clicking it triggers handleDownload with the recommendedModel id
    - Button disappears once a download starts (no duplicate UI)
    - Existing model card buttons remain functional
    - TypeScript compiles with no errors
  </done>
</task>

</tasks>

<verification>
1. `cargo check` passes in src-tauri (Rust compilation with new windows crate and DXGI code)
2. `npx tsc --noEmit` passes (TypeScript compilation of FirstRun changes)
3. Manual verification: on an NVIDIA system, check_first_run should return gpuDetected=true, directmlAvailable=false, recommendedModel="parakeet-tdt-v2-fp32"
4. Manual verification: FirstRun screen shows "Download Recommended" button above model cards
</verification>

<success_criteria>
- DXGI adapter enumeration detects discrete vs integrated GPUs by DedicatedVideoMemory threshold
- NVIDIA systems: recommend Parakeet (unchanged behavior)
- Discrete non-NVIDIA (AMD RX, Intel Arc): recommend Parakeet via DirectML
- Integrated-only (Intel UHD, AMD APU): recommend small-en, directml_available=false, Parakeet card hidden
- FirstRun UI has a prominent one-click "Download Recommended" button
- All existing model card functionality preserved
</success_criteria>

<output>
After completion, create `.planning/quick/25-auto-select-recommended-model-on-first-s/25-SUMMARY.md`
</output>
