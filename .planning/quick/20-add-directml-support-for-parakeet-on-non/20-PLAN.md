---
phase: quick-20
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/Cargo.toml
  - src-tauri/src/transcribe.rs
  - src-tauri/src/transcribe_parakeet.rs
  - src-tauri/src/lib.rs
  - src/App.tsx
  - src/components/FirstRun.tsx
  - src/components/sections/ModelSection.tsx
autonomous: true
requirements: [DIRECTML-01, GPU-STATUS-01]

must_haves:
  truths:
    - "Parakeet loads with CUDA EP on NVIDIA GPUs (existing behavior preserved)"
    - "Parakeet loads with DirectML EP on non-NVIDIA systems (Intel/AMD GPUs)"
    - "Parakeet falls back to CPU EP if neither CUDA nor DirectML is available"
    - "GPU status indicator shows detected GPU name, execution provider, and active model in settings ModelSection"
    - "FirstRun shows GPU badge with actual GPU name instead of just NVIDIA GPU Detected / CPU Mode"
    - "GPU status indicator updates after model selection changes"
  artifacts:
    - path: "src-tauri/Cargo.toml"
      provides: "directml feature added to parakeet-rs dependency"
      contains: "directml"
    - path: "src-tauri/src/transcribe.rs"
      provides: "Extended detect_gpu returning GPU name + provider recommendation"
      contains: "GpuDetection"
    - path: "src-tauri/src/transcribe_parakeet.rs"
      provides: "load_parakeet accepting provider string"
      contains: "provider"
    - path: "src-tauri/src/lib.rs"
      provides: "get_gpu_info Tauri command + CachedGpuDetection managed state"
      contains: "get_gpu_info"
    - path: "src/App.tsx"
      provides: "Updated FirstRunStatus type with gpuName and directmlAvailable, passes new props to FirstRun"
    - path: "src/components/sections/ModelSection.tsx"
      provides: "GPU status indicator with provider, GPU name, active model"
    - path: "src/components/FirstRun.tsx"
      provides: "GPU badge with actual GPU name and DirectML-aware model visibility"
  key_links:
    - from: "src-tauri/src/lib.rs"
      to: "src-tauri/src/transcribe_parakeet.rs"
      via: "load_parakeet call with correct provider based on CachedGpuDetection"
      pattern: "load_parakeet.*provider"
    - from: "src-tauri/src/lib.rs"
      to: "src-tauri/src/transcribe.rs"
      via: "detect_gpu_full returns GpuDetection struct consumed by CachedGpuDetection"
      pattern: "GpuDetection"
    - from: "src/components/sections/ModelSection.tsx"
      to: "get_gpu_info"
      via: "invoke('get_gpu_info') to populate status indicator"
      pattern: "invoke.*get_gpu_info"
    - from: "src/App.tsx"
      to: "src/components/FirstRun.tsx"
      via: "Passes gpuName and directmlAvailable from FirstRunStatus to FirstRun props"
      pattern: "gpuName.*directmlAvailable"
---

<objective>
Add DirectML execution provider support for Parakeet TDT on non-NVIDIA GPUs (Intel/AMD) and add a GPU/inference status indicator to the settings UI and FirstRun flow.

Purpose: Users with Intel or AMD GPUs currently get CPU-only Parakeet inference. DirectML enables GPU acceleration on any DirectX 12 GPU. The status indicator gives users visibility into what hardware and provider is actually being used.

Output: Backend DirectML EP selection with NVIDIA->CUDA / non-NVIDIA->DirectML / no-GPU->CPU fallback chain; frontend GPU status indicator in ModelSection and improved FirstRun GPU badge.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md

<interfaces>
<!-- Key types and contracts the executor needs. -->

From src-tauri/src/transcribe.rs:
```rust
#[derive(Debug, Clone, Copy)]
pub enum ModelMode {
    Gpu,
    Cpu,
}

pub fn detect_gpu() -> ModelMode {
    // Uses nvml_wrapper::Nvml to detect NVIDIA GPUs
    // Returns Gpu if NVIDIA found, Cpu otherwise
}

pub fn models_dir() -> PathBuf { ... }
```

From src-tauri/src/transcribe_parakeet.rs:
```rust
pub fn load_parakeet(model_dir: &str, use_cuda: bool) -> Result<ParakeetTDT, String>
// Currently: use_cuda=true -> CUDA EP, false -> CPU EP
```

From parakeet_rs (re-exports):
```rust
pub use execution::{ExecutionConfig, ExecutionProvider};
// ExecutionProvider::Cuda, ExecutionProvider::DirectML, ExecutionProvider::Cpu
```

From src-tauri/src/lib.rs:
```rust
pub struct CachedGpuMode(pub transcribe::ModelMode);
// Registered on Builder before run() — available to all commands

pub struct ActiveEngine(pub std::sync::Mutex<TranscriptionEngine>);
pub struct ParakeetStateMutex(pub std::sync::Mutex<Option<Arc<Mutex<ParakeetTDT>>>>);

// Parakeet load call sites (both pass use_cuda: true):
// 1. set_engine() at line 298: load_parakeet(&dir_str, true)
// 2. startup loader at line 1300: load_parakeet(&dir_str, true)
```

From src-tauri/Cargo.toml:
```toml
parakeet-rs = { version = "0.1.9", features = ["cuda"], optional = true }
```

From src-tauri/patches/parakeet-rs/Cargo.toml:
```toml
directml = ["ort/directml"]  # Feature already defined in patched crate
```

From src/App.tsx (lines 12-16, 27-36, 98-104):
```tsx
interface FirstRunStatus {
  needsSetup: boolean;
  gpuDetected: boolean;
  recommendedModel: string;
}

// Line 32: const status = await invoke<FirstRunStatus>('check_first_run');
// Line 36 (fallback): setFirstRunStatus({ needsSetup: false, gpuDetected: false, recommendedModel: '' });

// Lines 101-103: <FirstRun> usage:
//   gpuDetected={firstRunStatus.gpuDetected}
//   recommendedModel={firstRunStatus.recommendedModel}
```

From src/components/FirstRun.tsx:
```tsx
interface FirstRunProps {
  gpuDetected: boolean;        // true if NVIDIA GPU found
  recommendedModel: string;    // model id
  onComplete: (downloadedModelId: string) => void;
}
// GPU badge: "NVIDIA GPU Detected" or "CPU Mode"
// Models: large-v3-turbo and parakeet-tdt-v2-fp32 are gpuOnly: true
```

From src/components/sections/ModelSection.tsx:
```tsx
interface ModelSectionProps {
  selectedModel: string;
  onSelectedModelChange: (id: string) => void;
}
// No GPU status indicator currently exists
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Backend — DirectML EP support + GPU detection enrichment + get_gpu_info command</name>
  <files>
    src-tauri/Cargo.toml,
    src-tauri/src/transcribe.rs,
    src-tauri/src/transcribe_parakeet.rs,
    src-tauri/src/lib.rs
  </files>
  <action>
**1. Cargo.toml** (line 71): Add `"directml"` to parakeet-rs features:
```toml
parakeet-rs = { version = "0.1.9", features = ["cuda", "directml"], optional = true }
```

**2. transcribe.rs** — Add a new struct and function (keep existing `detect_gpu` unchanged):

Add above `detect_gpu()`:
```rust
/// Extended GPU detection result for provider selection and UI display.
#[derive(Debug, Clone)]
pub struct GpuDetection {
    /// Human-readable GPU name (e.g., "NVIDIA Quadro P2000", "DirectML (auto-detected)")
    pub gpu_name: String,
    /// Which execution provider to use for Parakeet: "cuda", "directml", or "cpu"
    pub parakeet_provider: String,
    /// Whether this is an NVIDIA GPU (for Whisper CUDA and ModelMode::Gpu)
    pub is_nvidia: bool,
}
```

Add below `detect_gpu()`:
```rust
pub fn detect_gpu_full() -> GpuDetection {
    use nvml_wrapper::Nvml;
    match Nvml::init() {
        Ok(nvml) => match nvml.device_by_index(0) {
            Ok(device) => {
                let name = device.name().unwrap_or_else(|_| "Unknown NVIDIA GPU".to_string());
                log::info!("GPU detection (full): NVIDIA GPU found: {}", name);
                GpuDetection {
                    gpu_name: name,
                    parakeet_provider: "cuda".to_string(),
                    is_nvidia: true,
                }
            }
            Err(e) => {
                log::info!("GPU detection (full): NVML init OK but no device: {} — using DirectML", e);
                GpuDetection {
                    gpu_name: "DirectML (auto-detected)".to_string(),
                    parakeet_provider: "directml".to_string(),
                    is_nvidia: false,
                }
            }
        },
        Err(e) => {
            log::info!("GPU detection (full): NVML failed: {} — using DirectML", e);
            GpuDetection {
                gpu_name: "DirectML (auto-detected)".to_string(),
                parakeet_provider: "directml".to_string(),
                is_nvidia: false,
            }
        }
    }
}
```

**3. transcribe_parakeet.rs** — Change `load_parakeet` signature from `(model_dir: &str, use_cuda: bool)` to `(model_dir: &str, provider: &str)`:

Replace the entire function body to match on `provider`:
- `"cuda"` -> `Some(ExecutionConfig::new().with_execution_provider(ExecutionProvider::Cuda))`
- `"directml"` -> `Some(ExecutionConfig::new().with_execution_provider(ExecutionProvider::DirectML))`
- anything else -> `None` (CPU default)

Update all logging to reference `provider` instead of `use_cuda`. Update doc comment to reflect the new signature.

**4. lib.rs** — Multiple changes:

a) Add `CachedGpuDetection` managed state struct next to `CachedGpuMode` (around line 112):
```rust
#[cfg(feature = "whisper")]
pub struct CachedGpuDetection(pub transcribe::GpuDetection);
```

b) In the builder section (around line 1176), compute full detection alongside existing `cached_gpu`:
```rust
#[cfg(feature = "whisper")]
let cached_gpu_detection = {
    let detection = transcribe::detect_gpu_full();
    log::info!("GPU detection full: {:?}", detection);
    detection
};
```
Register on builder (around line 1198, after CachedGpuMode):
```rust
builder = builder.manage(CachedGpuDetection(cached_gpu_detection));
```

c) Update `set_engine()` (line 298) — replace `load_parakeet(&dir_str, true)` with:
```rust
let gpu_detection = app.state::<CachedGpuDetection>();
let provider = gpu_detection.0.parakeet_provider.as_str();
transcribe_parakeet::load_parakeet(&dir_str, provider)
```

d) Update startup Parakeet loader (line 1300) — same change:
```rust
let gpu_detection = app.state::<CachedGpuDetection>();
let provider = gpu_detection.0.parakeet_provider.as_str();
transcribe_parakeet::load_parakeet(&dir_str, provider)
```

e) Add `GpuInfo` struct and `get_gpu_info` Tauri command (place near `check_first_run`):
```rust
#[cfg(feature = "whisper")]
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct GpuInfo {
    gpu_name: String,
    execution_provider: String,
    active_model: String,
    active_engine: String,
}

#[cfg(feature = "whisper")]
#[tauri::command]
fn get_gpu_info(app: tauri::AppHandle) -> GpuInfo {
    let detection = app.state::<CachedGpuDetection>();
    let engine_state = app.state::<ActiveEngine>();
    let engine = engine_state.0.lock().unwrap_or_else(|e| e.into_inner());
    let engine_str = match *engine {
        TranscriptionEngine::Whisper => "whisper",
        TranscriptionEngine::Parakeet => "parakeet",
    };
    let settings = read_settings(&app).unwrap_or_else(|_| serde_json::json!({}));
    let active_model = if *engine == TranscriptionEngine::Parakeet {
        settings.get("parakeet_model").and_then(|v| v.as_str()).unwrap_or("parakeet-tdt-v2-fp32").to_string()
    } else {
        settings.get("whisper_model_id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string()
    };
    let ep = match engine_str {
        "parakeet" => detection.0.parakeet_provider.to_uppercase(),
        _ => if detection.0.is_nvidia { "CUDA".to_string() } else { "CPU".to_string() },
    };
    GpuInfo {
        gpu_name: detection.0.gpu_name.clone(),
        execution_provider: ep,
        active_model,
        active_engine: engine_str.to_string(),
    }
}
```

f) Register `get_gpu_info` in `invoke_handler` (after `check_first_run`):
```rust
#[cfg(feature = "whisper")]
get_gpu_info,
```

g) Update `FirstRunStatus` struct — add `gpu_name` and `directml_available`:
```rust
struct FirstRunStatus {
    needs_setup: bool,
    gpu_detected: bool,
    gpu_name: String,
    directml_available: bool,
    recommended_model: String,
}
```

h) Update `check_first_run` body to populate new fields:
```rust
let detection = app.state::<CachedGpuDetection>();
// ... existing logic ...
FirstRunStatus {
    needs_setup: !large_exists && !small_exists && !parakeet_fp32_exists,
    gpu_detected: gpu_mode,
    gpu_name: detection.0.gpu_name.clone(),
    directml_available: !gpu_mode,  // Non-NVIDIA systems get DirectML for Parakeet
    recommended_model: if gpu_mode {
        "parakeet-tdt-v2-fp32".to_string()
    } else {
        "small-en".to_string()
    },
}
```
  </action>
  <verify>
    <automated>cd src-tauri && cargo check --features "whisper,parakeet" 2>&1 | tail -5</automated>
  </verify>
  <done>
    - `cargo check` passes with both features enabled
    - `load_parakeet` accepts a provider string ("cuda", "directml", "cpu")
    - `detect_gpu_full()` returns `GpuDetection` with gpu_name, parakeet_provider, is_nvidia
    - `CachedGpuDetection` is registered as managed state on builder
    - Both `load_parakeet` call sites use cached provider from `CachedGpuDetection`
    - `get_gpu_info` command returns GpuInfo struct with gpu_name, execution_provider, active_model, active_engine
    - `check_first_run` returns gpu_name and directml_available fields
    - directml feature is in parakeet-rs dependency features list
  </done>
</task>

<task type="auto">
  <name>Task 2: Frontend — GPU status indicator in ModelSection + DirectML-aware FirstRun</name>
  <files>
    src/App.tsx,
    src/components/sections/ModelSection.tsx,
    src/components/FirstRun.tsx
  </files>
  <action>
**1. App.tsx** — Update `FirstRunStatus` interface and `<FirstRun>` props:

a) Update the `FirstRunStatus` interface (line 12) to add new fields:
```tsx
interface FirstRunStatus {
  needsSetup: boolean;
  gpuDetected: boolean;
  gpuName: string;
  directmlAvailable: boolean;
  recommendedModel: string;
}
```

b) Update the fallback on line 36:
```tsx
setFirstRunStatus({ needsSetup: false, gpuDetected: false, gpuName: '', directmlAvailable: false, recommendedModel: '' });
```

c) Update the `<FirstRun>` JSX (around line 101) to pass new props:
```tsx
<FirstRun
  gpuDetected={firstRunStatus.gpuDetected}
  gpuName={firstRunStatus.gpuName}
  directmlAvailable={firstRunStatus.directmlAvailable}
  recommendedModel={firstRunStatus.recommendedModel}
  onComplete={...}
/>
```

**2. ModelSection.tsx** — Add GPU status indicator below the ModelSelector:

a) Add a `GpuInfo` type at the top:
```tsx
interface GpuInfo {
  gpuName: string;
  executionProvider: string;
  activeModel: string;
  activeEngine: string;
}
```

b) Add state and effect inside `ModelSection` component:
```tsx
const [gpuInfo, setGpuInfo] = useState<GpuInfo | null>(null);

useEffect(() => {
  invoke<GpuInfo>('get_gpu_info').then(setGpuInfo).catch(console.error);
}, [selectedModel, currentEngine]);
```

c) Render between `<ModelSelector>` and the Parakeet note `<p>`:
```tsx
{gpuInfo && (
  <div className="mt-4 rounded-lg border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50 px-4 py-3">
    <p className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider mb-2">
      Inference Status
    </p>
    <div className="space-y-1">
      <div className="flex justify-between text-sm">
        <span className="text-gray-500 dark:text-gray-400">GPU</span>
        <span className="text-gray-900 dark:text-gray-100 font-medium">{gpuInfo.gpuName}</span>
      </div>
      <div className="flex justify-between text-sm">
        <span className="text-gray-500 dark:text-gray-400">Provider</span>
        <span className="text-gray-900 dark:text-gray-100 font-medium">{gpuInfo.executionProvider}</span>
      </div>
      <div className="flex justify-between text-sm">
        <span className="text-gray-500 dark:text-gray-400">Engine</span>
        <span className="text-gray-900 dark:text-gray-100 font-medium capitalize">{gpuInfo.activeEngine}</span>
      </div>
    </div>
  </div>
)}
```

**3. FirstRun.tsx** — Make DirectML-aware:

a) Update `FirstRunProps`:
```tsx
interface FirstRunProps {
  gpuDetected: boolean;
  gpuName: string;
  directmlAvailable: boolean;
  recommendedModel: string;
  onComplete: (downloadedModelId: string) => void;
}
```

b) Destructure new props: `{ gpuDetected, gpuName, directmlAvailable, recommendedModel, onComplete }`

c) Update GPU detection badge (around line 181-193). Replace the existing two-branch conditional with three branches:
- If `gpuDetected` (NVIDIA): green badge showing `gpuName` (e.g., "NVIDIA Quadro P2000")
- Else if `directmlAvailable`: blue badge showing "GPU Detected (DirectML)" with a blue dot
- Else: gray badge "CPU Mode" (unchanged)

```tsx
{gpuDetected ? (
  <span className="inline-flex items-center gap-1.5 rounded-full bg-green-100 px-3 py-1 text-sm font-medium text-green-700 dark:bg-green-900/40 dark:text-green-400">
    <span className="h-2 w-2 rounded-full bg-green-500" />
    {gpuName}
  </span>
) : directmlAvailable ? (
  <span className="inline-flex items-center gap-1.5 rounded-full bg-blue-100 px-3 py-1 text-sm font-medium text-blue-700 dark:bg-blue-900/40 dark:text-blue-400">
    <span className="h-2 w-2 rounded-full bg-blue-500" />
    GPU Detected (DirectML)
  </span>
) : (
  <span className="inline-flex items-center gap-1.5 rounded-full bg-gray-100 px-3 py-1 text-sm font-medium text-gray-600 dark:bg-gray-800 dark:text-gray-400">
    <span className="h-2 w-2 rounded-full bg-gray-400" />
    CPU Mode
  </span>
)}
```

d) Update MODELS array — change Parakeet entry:
```tsx
{
  id: 'parakeet-tdt-v2-fp32',
  name: 'Parakeet TDT (fp32)',
  size: '2.56 GB',
  quality: 'Full precision (GPU)',
  requirement: 'Requires GPU (CUDA or DirectML)',
  gpuOnly: false,  // Now handled by custom filter
}
```

e) Replace `visibleModels` filter (line 57):
```tsx
const visibleModels = MODELS.filter((m) => {
  if (m.id === 'parakeet-tdt-v2-fp32') {
    return gpuDetected || directmlAvailable;
  }
  if (m.gpuOnly) {
    return gpuDetected;
  }
  return true;
});
```

f) Update grid class logic — the grid columns should adapt to `visibleModels.length`:
```tsx
const gridClass =
  visibleModels.length >= 3
    ? 'grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3 mb-6'
    : visibleModels.length === 2
      ? 'grid grid-cols-1 gap-4 sm:grid-cols-2 mb-6'
      : 'grid grid-cols-1 gap-4 mb-6';
```
  </action>
  <verify>
    <automated>npx tsc --noEmit 2>&1 | tail -5</automated>
  </verify>
  <done>
    - App.tsx FirstRunStatus type includes gpuName and directmlAvailable
    - App.tsx passes gpuName and directmlAvailable props to FirstRun
    - ModelSection shows "Inference Status" block with GPU name, execution provider, and engine
    - GPU info refreshes when model selection or engine changes
    - FirstRun GPU badge shows actual GPU name for NVIDIA, "GPU Detected (DirectML)" for non-NVIDIA
    - FirstRun shows Parakeet card for DirectML users (non-NVIDIA GPUs)
    - Whisper GPU models (large-v3-turbo) remain hidden for non-NVIDIA users
    - TypeScript compiles without errors
  </done>
</task>

</tasks>

<verification>
1. `cargo check --features "whisper,parakeet"` passes
2. `npx tsc --noEmit` passes
3. `get_gpu_info` command is registered and returns GpuInfo struct
4. `load_parakeet` call sites pass provider string from CachedGpuDetection, not hardcoded `true`
5. DirectML feature is present in Cargo.toml parakeet-rs features
6. FirstRun correctly shows/hides Parakeet card based on directmlAvailable OR gpuDetected
</verification>

<success_criteria>
- Backend compiles with both cuda and directml features on parakeet-rs
- Parakeet provider selection follows: NVIDIA -> CUDA, non-NVIDIA -> DirectML, no-GPU -> CPU
- get_gpu_info returns gpu_name, execution_provider, active_model, active_engine
- ModelSection displays inference status indicator
- FirstRun shows GPU name and makes Parakeet available to DirectML users
- No regression: NVIDIA users still get CUDA for both Whisper and Parakeet
</success_criteria>

<output>
After completion, create `.planning/quick/20-add-directml-support-for-parakeet-on-non/20-SUMMARY.md`
</output>
