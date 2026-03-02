---
phase: quick-13
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/src/download.rs
  - src-tauri/src/lib.rs
  - src/components/sections/ModelSection.tsx
  - src/components/ModelSelector.tsx
  - src/components/FirstRun.tsx
autonomous: true
requirements: [QUICK-13]

must_haves:
  truths:
    - "User can see both 'Parakeet TDT (int8)' and 'Parakeet TDT (fp32)' as separate entries in settings model list and first-run flow"
    - "User can download the fp32 model independently of the int8 model"
    - "Selecting fp32 variant sets engine to parakeet and loads from the fp32 model directory"
    - "Switching between int8 and fp32 reloads the Parakeet model from the correct directory"
    - "Both variants can be downloaded and exist on disk simultaneously"
  artifacts:
    - path: "src-tauri/src/download.rs"
      provides: "fp32 file list, download command, dir helper, exists check"
      contains: "PARAKEET_FP32_FILES"
    - path: "src-tauri/src/lib.rs"
      provides: "fp32 ModelInfo entry in list_models, parakeet_model setting for variant resolution, variant-aware set_engine and startup loading"
      contains: "parakeet-tdt-v2-fp32"
    - path: "src/components/sections/ModelSection.tsx"
      provides: "fp32 download state management and handler"
      contains: "parakeet-tdt-v2-fp32"
    - path: "src/components/ModelSelector.tsx"
      provides: "fp32-aware Parakeet card rendering with download buttons"
      contains: "parakeet-tdt-v2-fp32"
    - path: "src/components/FirstRun.tsx"
      provides: "fp32 entry in MODELS array"
      contains: "parakeet-tdt-v2-fp32"
  key_links:
    - from: "src/components/sections/ModelSection.tsx"
      to: "download::download_parakeet_fp32_model"
      via: "invoke('download_parakeet_fp32_model')"
      pattern: "download_parakeet_fp32_model"
    - from: "src/components/sections/ModelSection.tsx"
      to: "lib.rs set_engine"
      via: "handleModelSelect always calls set_engine with parakeetModel for parakeet variants, even if engine == currentEngine"
      pattern: "parakeet_model"
    - from: "lib.rs set_engine"
      to: "download::parakeet_fp32_model_dir"
      via: "reads parakeet_model from settings to resolve correct model directory"
      pattern: "parakeet_fp32_model_dir\\(\\)|parakeet_model_dir\\(\\)"
---

<objective>
Add fp32 Parakeet TDT model as a second selectable variant alongside the existing int8 model.

Purpose: Enable accuracy comparison between int8 (652 MB, quantized) and fp32 (2.56 GB, full precision) Parakeet models. Both come from the same HuggingFace repo (istupakov/parakeet-tdt-0.6b-v2-onnx).

Output: Users see two Parakeet entries in settings and first-run, can download either/both independently, and switching between them reloads the model from the correct directory.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src-tauri/src/download.rs
@src-tauri/src/lib.rs
@src-tauri/src/transcribe_parakeet.rs
@src/components/sections/ModelSection.tsx
@src/components/ModelSelector.tsx
@src/components/FirstRun.tsx

<interfaces>
<!-- Key types and contracts the executor needs -->

From src-tauri/src/download.rs:
```rust
// Existing pattern — PARAKEET_FILES const + parakeet_model_dir() + parakeet_model_exists()
// + download_parakeet_model Tauri command. fp32 follows same pattern.
const PARAKEET_FILES: &[(&str, &str, u64)] = &[...]; // (remote_name, local_name, size)
pub fn parakeet_model_dir() -> PathBuf  // models_dir().join("parakeet-tdt-v2")
pub fn parakeet_model_exists() -> bool  // checks encoder-model.onnx exists
pub async fn download_parakeet_model(on_event: Channel<DownloadEvent>) -> Result<(), String>
```

From src-tauri/src/lib.rs:
```rust
// ModelInfo returned by list_models
struct ModelInfo { id, name, description, recommended, downloaded }

// set_engine resolves model dir via download::parakeet_model_dir()
// Currently hardcoded — needs to resolve based on which variant is selected

// read_settings / write_settings helpers for settings.json persistence

// ActiveEngine managed state — Whisper or Parakeet (no variant distinction yet)
// ParakeetStateMutex — holds loaded model, needs reload on variant switch
```

From frontend:
```typescript
// ModelSection passes parakeet-specific download props to ModelSelector
// ModelSelector checks `model.id === 'parakeet-tdt-v2'` for isParakeet logic
// FirstRun has hardcoded MODELS array with id/name/size/quality/requirement/gpuOnly
// All three use invoke('download_parakeet_model') for parakeet downloads
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Backend — fp32 download infrastructure and variant-aware engine dispatch</name>
  <files>src-tauri/src/download.rs, src-tauri/src/lib.rs</files>
  <action>
**download.rs — Add fp32 download support (mirror existing int8 pattern):**

1. Add `PARAKEET_FP32_FILES` constant after `PARAKEET_FILES`:
```rust
/// Parakeet TDT fp32 ONNX files from HuggingFace repo istupakov/parakeet-tdt-0.6b-v2-onnx.
///
/// fp32 uses ONNX external data format: encoder-model.onnx is a small header (~42MB),
/// encoder-model.onnx.data contains the actual weights (~2.44GB). Both must be co-located.
/// Remote filenames match local filenames (no renaming needed — unlike int8 which has .int8. prefix).
const PARAKEET_FP32_FILES: &[(&str, &str, u64)] = &[
    ("encoder-model.onnx", "encoder-model.onnx", 41_800_000),
    ("encoder-model.onnx.data", "encoder-model.onnx.data", 2_440_000_000),
    ("decoder_joint-model.onnx", "decoder_joint-model.onnx", 35_800_000),
    ("nemo128.onnx", "nemo128.onnx", 139_764),
    ("vocab.txt", "vocab.txt", 9_384),
    ("config.json", "config.json", 97),
];
```
NOTE on sizes: The encoder-model.onnx size (41.8MB) and encoder-model.onnx.data size (2.44GB) and decoder size (35.8MB) are estimates. They will be refined after first download. The download function does not validate sizes (no SHA256 for Parakeet — decision from phase 08), it just uses them for progress bar denominator. content-length fallback applies.

2. Add `parakeet_fp32_model_dir()`:
```rust
pub fn parakeet_fp32_model_dir() -> PathBuf {
    models_dir().join("parakeet-tdt-v2-fp32")
}
```

3. Add `parakeet_fp32_model_exists()`:
```rust
pub fn parakeet_fp32_model_exists() -> bool {
    parakeet_fp32_model_dir()
        .join("encoder-model.onnx")
        .exists()
}
```

4. Add `download_parakeet_fp32_model` Tauri command — clone the `download_parakeet_model` function body, changing:
   - Function name: `download_parakeet_fp32_model`
   - File list: `PARAKEET_FP32_FILES` instead of `PARAKEET_FILES`
   - Directory: `parakeet_fp32_model_dir()` instead of `parakeet_model_dir()`
   - Log messages: reference "fp32" variant
   - Started event url string: `"parakeet-tdt-v2-fp32 (6 files)"` (6 files, not 5 — includes .onnx.data)

**lib.rs — Variant-aware engine dispatch:**

5. Add helper function `read_saved_parakeet_model` (near `read_saved_engine`):
```rust
/// Read the saved Parakeet model variant from settings.json.
/// Returns "parakeet-tdt-v2" (int8) by default.
fn read_saved_parakeet_model(app_handle: &tauri::AppHandle) -> String {
    let json = read_settings(app_handle).unwrap_or_else(|_| serde_json::json!({}));
    json.get("parakeet_model")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("parakeet-tdt-v2")
        .to_string()
}
```

Also add an overload that accepts `&tauri::App` for startup use (same as `read_saved_engine` pattern):
```rust
fn read_saved_parakeet_model_startup(app: &tauri::App) -> String {
    let data_dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(_) => return "parakeet-tdt-v2".to_string(),
    };
    let settings_path = data_dir.join("settings.json");
    let contents = match std::fs::read_to_string(&settings_path) {
        Ok(c) => c,
        Err(_) => return "parakeet-tdt-v2".to_string(),
    };
    let json: serde_json::Value = match serde_json::from_str(&contents) {
        Ok(j) => j,
        Err(_) => return "parakeet-tdt-v2".to_string(),
    };
    json.get("parakeet_model")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("parakeet-tdt-v2")
        .to_string()
}
```

6. Add helper `resolve_parakeet_dir` to map model ID to directory:
```rust
/// Resolve the model directory for a Parakeet variant.
fn resolve_parakeet_dir(model_id: &str) -> std::path::PathBuf {
    match model_id {
        "parakeet-tdt-v2-fp32" => download::parakeet_fp32_model_dir(),
        _ => download::parakeet_model_dir(), // default to int8
    }
}
```

7. Update `list_models()` — add fp32 entry after the int8 entry:
```rust
models.push(ModelInfo {
    id: "parakeet-tdt-v2-fp32".to_string(),
    name: "Parakeet TDT (fp32)".to_string(),
    description: "Full precision — 2.56 GB — requires NVIDIA GPU (ONNX)".to_string(),
    recommended: false,
    downloaded: crate::download::parakeet_fp32_model_exists(),
});
```
Also update the existing int8 entry name from "Parakeet TDT" to "Parakeet TDT (int8)" so both are distinguishable.

8. Update `set_engine()` — change the function signature and replace the is_none block:

   **Step 8a — Change the function signature** to accept an optional parakeet_model parameter:
   ```rust
   #[tauri::command]
   fn set_engine(app: tauri::AppHandle, engine: String, parakeet_model: Option<String>) -> Result<(), String> {
   ```
   Tauri IPC deserializes missing fields as None for Option types, so existing callers that omit `parakeet_model` will receive None and the backend falls back to reading from settings.json.

   **Step 8b — Persist parakeet_model to settings.json** when provided. In the settings write block (currently near line 288, after the Parakeet load block), persist the variant before writing `active_engine`:
   ```rust
   // Persist to settings.json
   let mut json = read_settings(&app)?;
   if let Some(ref variant) = parakeet_model {
       json["parakeet_model"] = serde_json::Value::String(variant.clone());
   }
   json["active_engine"] = serde_json::Value::String(engine);
   write_settings(&app, &json)?;
   ```

   **Step 8c — Replace the is_none block** (currently lines 250–285) with an unconditional reload that reads the resolved variant. Delete the entire existing block:
   ```rust
   // DELETE THIS BLOCK (lines 250–285):
   let is_none = {
       let guard = parakeet_state.0.lock().unwrap_or_else(|e| e.into_inner());
       guard.is_none()
   };
   if is_none {
       let model_dir = download::parakeet_model_dir();
       if model_dir.exists() {
           ...
       } else {
           ...
       }
   }
   ```

   Replace it with this unconditional block:
   ```rust
   // REPLACE WITH (always reload on any parakeet switch, including variant changes):
   let parakeet_model_id = parakeet_model
       .clone()
       .unwrap_or_else(|| read_saved_parakeet_model(&app));
   let model_dir = resolve_parakeet_dir(&parakeet_model_id);
   if model_dir.exists() {
       let dir_str = model_dir.to_string_lossy().to_string();
       match transcribe_parakeet::load_parakeet(&dir_str, true) {
           Ok(p) => {
               let mut guard = parakeet_state.0.lock().unwrap_or_else(|e| e.into_inner());
               *guard = Some(std::sync::Arc::new(std::sync::Mutex::new(p)));
               log::info!("Parakeet model loaded on engine switch (variant: {})", parakeet_model_id);
           }
           Err(e) => {
               log::error!("Failed to load Parakeet on engine switch: {}", e);
               // Revert to Whisper since Parakeet failed
               let state = app.state::<ActiveEngine>();
               let mut guard = state.0.lock().unwrap_or_else(|e| e.into_inner());
               *guard = TranscriptionEngine::Whisper;
               return Err(format!(
                   "Parakeet model failed to load: {}. Reverting to Whisper.",
                   e
               ));
           }
       }
   } else {
       // Revert — model not downloaded
       let state = app.state::<ActiveEngine>();
       let mut guard = state.0.lock().unwrap_or_else(|e| e.into_inner());
       *guard = TranscriptionEngine::Whisper;
       return Err(
           "Parakeet model not downloaded. Download it first from Settings.".to_string(),
       );
   }
   ```

9. Update `check_first_run()` — also check fp32 existence:
```rust
let parakeet_fp32_exists = crate::download::parakeet_fp32_model_exists();
FirstRunStatus {
    needs_setup: !large_exists && !small_exists && !parakeet_exists && !parakeet_fp32_exists,
    ...
}
```

10. Update startup Parakeet loading in `setup()` — use variant-aware dir:
```rust
if saved_engine == TranscriptionEngine::Parakeet {
    let parakeet_model_id = read_saved_parakeet_model_startup(app);
    let model_dir = resolve_parakeet_dir(&parakeet_model_id);
    if model_dir.exists() {
        let dir_str = model_dir.to_string_lossy().to_string();
        match transcribe_parakeet::load_parakeet(&dir_str, true) {
            ...
        }
    }
}
```

11. Register `download::download_parakeet_fp32_model` in the `invoke_handler` macro alongside `download::download_parakeet_model`.
  </action>
  <verify>
    <automated>cd src-tauri && cargo check --features whisper,parakeet 2>&1 | tail -5</automated>
  </verify>
  <done>
    - PARAKEET_FP32_FILES constant with 6 entries (encoder header, encoder data, decoder, preprocessor, vocab, config)
    - parakeet_fp32_model_dir() returns models/parakeet-tdt-v2-fp32
    - parakeet_fp32_model_exists() checks encoder-model.onnx in fp32 dir
    - download_parakeet_fp32_model Tauri command registered and compiles
    - list_models returns "parakeet-tdt-v2-fp32" entry with downloaded status
    - set_engine signature is fn set_engine(app: tauri::AppHandle, engine: String, parakeet_model: Option<String>) -> Result<(), String>
    - set_engine persists parakeet_model to settings.json when Some is provided
    - set_engine always reloads Parakeet model on parakeet switch (no is_none check)
    - Startup loading uses variant-aware dir resolution
    - check_first_run considers fp32 existence
    - cargo check passes with both features
  </done>
</task>

<task type="auto">
  <name>Task 2: Frontend — fp32 model cards, download handlers, and variant selection</name>
  <files>src/components/sections/ModelSection.tsx, src/components/ModelSelector.tsx, src/components/FirstRun.tsx</files>
  <action>
**ModelSection.tsx — Add fp32 download state and handler:**

1. Add state variables for fp32 download (mirror the existing parakeet int8 pattern):
```typescript
const [fp32Downloading, setFp32Downloading] = useState(false);
const [fp32Percent, setFp32Percent] = useState(0);
const [fp32Error, setFp32Error] = useState<string | null>(null);
```

2. Add `handleFp32Download` function — same pattern as `handleParakeetDownload` but invokes `download_parakeet_fp32_model`:
```typescript
async function handleFp32Download() {
    setFp32Downloading(true);
    setFp32Percent(0);
    setFp32Error(null);

    const onEvent = new Channel<DownloadEvent>();
    onEvent.onmessage = async (msg) => {
      switch (msg.event) {
        case 'started': break;
        case 'progress': {
          const pct = msg.data.totalBytes > 0
            ? Math.round((msg.data.downloadedBytes / msg.data.totalBytes) * 100) : 0;
          setFp32Percent(pct);
          break;
        }
        case 'finished':
          setFp32Downloading(false);
          await loadModels();
          await handleModelSelect('parakeet-tdt-v2-fp32');
          break;
        case 'error':
          setFp32Error(msg.data.message);
          setFp32Downloading(false);
          break;
      }
    };

    try {
      await invoke('download_parakeet_fp32_model', { onEvent });
    } catch (e) {
      setFp32Error(String(e));
      setFp32Downloading(false);
    }
}
```

3. Update `handleModelSelect` — both parakeet variants always call `set_engine`, even when already on the parakeet engine. This is required because switching from int8 to fp32 (or vice versa) while the engine is already "parakeet" must trigger a model reload via set_engine:

```typescript
async function handleModelSelect(modelId: string) {
    const isParakeetVariant = modelId === 'parakeet-tdt-v2' || modelId === 'parakeet-tdt-v2-fp32';
    const engine = isParakeetVariant ? 'parakeet' : 'whisper';

    if (isParakeetVariant) {
        // Always call set_engine for Parakeet variants regardless of currentEngine.
        // This is necessary because variant switches (int8 -> fp32 or fp32 -> int8)
        // require a model reload even when the engine is already "parakeet".
        await invoke('set_engine', { engine: 'parakeet', parakeetModel: modelId });
    } else {
        if (engine !== currentEngine) {
            await invoke('set_engine', { engine, parakeetModel: null });
        }
        await invoke('set_model', { modelId });
    }
}
```

4. Pass fp32 props to ModelSelector:
```typescript
<ModelSelector
    ...existing props...
    onFp32Download={handleFp32Download}
    fp32Downloading={fp32Downloading}
    fp32Percent={fp32Percent}
    fp32Error={fp32Error}
/>
```

5. Filter whisperModels: update the filter to exclude BOTH parakeet variants:
```typescript
// Currently: models.filter(m => m.id !== 'parakeet-tdt-v2')
// Change to:
const whisperModels = models.filter(m => !m.id.startsWith('parakeet-'));
```
(This is only relevant if such filtering exists — check the actual render. The current code passes all models to ModelSelector, but the Parakeet note renders conditionally on `currentEngine === 'parakeet'`. No whisper filtering seems to happen in ModelSection directly — it's all handled in ModelSelector.)

**ModelSelector.tsx — fp32-aware card rendering:**

6. Update `ModelSelectorProps` interface — add fp32 props:
```typescript
interface ModelSelectorProps {
    ...existing...
    onFp32Download?: () => void;
    fp32Downloading?: boolean;
    fp32Percent?: number;
    fp32Error?: string | null;
}
```

7. Destructure new props with defaults in the component.

8. Update the `isParakeet` logic in the render loop. Currently `const isParakeet = model.id === 'parakeet-tdt-v2'`. Change to handle both variants:
```typescript
const isParakeetInt8 = model.id === 'parakeet-tdt-v2';
const isParakeetFp32 = model.id === 'parakeet-tdt-v2-fp32';
const isParakeet = isParakeetInt8 || isParakeetFp32;
```

9. For fp32 card: use fp32-specific download handler, downloading state, percent, and error. The existing code uses `onParakeetDownload`, `parakeetDownloading`, `parakeetPercent`, `parakeetError` for int8. For fp32, use `onFp32Download`, `fp32Downloading`, `fp32Percent`, `fp32Error`.

Update the conditional rendering inside the map:
```typescript
const thisDownloading = isParakeetInt8 ? parakeetDownloading : isParakeetFp32 ? fp32Downloading : false;
const thisPercent = isParakeetInt8 ? parakeetPercent : isParakeetFp32 ? fp32Percent : 0;
const thisError = isParakeetInt8 ? parakeetError : isParakeetFp32 ? fp32Error : null;
const thisOnDownload = isParakeetInt8 ? onParakeetDownload : isParakeetFp32 ? onFp32Download : undefined;
```

Then use these local variables in place of the hardcoded parakeet references:
- `isParakeetDownloading` -> `thisDownloading` (when isParakeet)
- `parakeetPercent` -> `thisPercent` (when isParakeet)
- `parakeetError` -> `thisError` (when isParakeet)
- `onParakeetDownload` -> `thisOnDownload` (when isParakeet)

The download button, progress bar, and error rendering for Parakeet cards should use these resolved variables. The isParakeet guard on the JSX stays the same (both variants render as "Parakeet-style" cards with dashed border and direct download button).

10. Update `disabled` calculation — also check fp32Downloading:
```typescript
const disabled = !model.downloaded || loadingId !== null || downloadingId !== null || parakeetDownloading || fp32Downloading;
```

**FirstRun.tsx — Add fp32 entry:**

11. Add fp32 to the MODELS array:
```typescript
{
    id: 'parakeet-tdt-v2-fp32',
    name: 'Parakeet TDT (fp32)',
    size: '2.56 GB',
    quality: 'Full precision (GPU)',
    requirement: 'Requires NVIDIA GPU',
    gpuOnly: true,
},
```
Also rename the existing int8 entry from "Parakeet TDT" to "Parakeet TDT (int8)".

12. Update the `handleDownload` function dispatch — fp32 variant uses `download_parakeet_fp32_model`:
```typescript
if (modelId === 'parakeet-tdt-v2') {
    await invoke('download_parakeet_model', { onEvent });
} else if (modelId === 'parakeet-tdt-v2-fp32') {
    await invoke('download_parakeet_fp32_model', { onEvent });
} else {
    await invoke('download_model', { modelId, onEvent });
}
```

13. Update the post-download `handleComplete` effect — both parakeet variants should activate parakeet engine. Pass the variant as `parakeetModel`:
```typescript
if (downloadingId === 'parakeet-tdt-v2' || downloadingId === 'parakeet-tdt-v2-fp32') {
    try {
        await invoke('set_engine', { engine: 'parakeet', parakeetModel: downloadingId });
    } catch (e) {
        console.warn('Failed to set Parakeet engine:', e);
    }
}
```

14. Update the `isFastest` badge logic:
```typescript
const isFastest = model.id === 'parakeet-tdt-v2' && gpuDetected;
```
Keep this for int8 only (it is the fastest due to quantization). Do NOT add "Fastest" to fp32.

15. Update the grid class — with 4 GPU models, use `sm:grid-cols-4` when gpuDetected (or keep `sm:grid-cols-3` and let them wrap since 4 cols may be too narrow):
```typescript
const gridClass = gpuDetected
    ? 'grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4 mb-6'
    : 'grid grid-cols-1 gap-4 mb-6';
```
  </action>
  <verify>
    <automated>cd src-tauri && cargo check --features whisper,parakeet 2>&1 | tail -5 && cd .. && npx tsc --noEmit 2>&1 | tail -10</automated>
  </verify>
  <done>
    - Both "Parakeet TDT (int8)" and "Parakeet TDT (fp32)" appear in list_models output
    - FirstRun shows fp32 card with 2.56 GB size when GPU detected
    - ModelSelector renders fp32 card with download button, progress bar, and error handling
    - ModelSection manages fp32 download state independently from int8
    - Selecting either parakeet variant always calls set_engine with parakeetModel parameter (regardless of currentEngine)
    - TypeScript compiles without errors
    - Rust compiles without errors
  </done>
</task>

<task type="auto">
  <name>Task 3: Integration verification — set_engine parakeet_model parameter and variant switching</name>
  <files>src-tauri/src/lib.rs</files>
  <action>
This task verifies the critical integration point: the `set_engine` command must accept the optional `parakeet_model` parameter correctly.

1. Verify the `set_engine` Tauri command signature compiles with the optional parameter:
```rust
#[tauri::command]
fn set_engine(app: tauri::AppHandle, engine: String, parakeet_model: Option<String>) -> Result<(), String> {
```
Tauri IPC deserializes missing fields as None for Option types, so existing frontend callers that don't pass `parakeet_model` will work (it defaults to None, and the backend falls back to reading from settings.json).

2. Verify that when `parakeet_model` is `Some(variant)`, it is persisted to settings.json (implemented in Task 1 step 8b):
```rust
// Inside set_engine, in the settings write block:
if let Some(ref variant) = parakeet_model {
    json["parakeet_model"] = serde_json::Value::String(variant.clone());
}
json["active_engine"] = serde_json::Value::String(engine);
write_settings(&app, &json)?;
```

3. Verify that variant switching works: if the user has int8 loaded and selects fp32, `set_engine` should:
   - Receive `parakeet_model = Some("parakeet-tdt-v2-fp32")`
   - Persist "parakeet-tdt-v2-fp32" to settings.json
   - Resolve dir → models/parakeet-tdt-v2-fp32
   - Unconditionally reload (no is_none check — implemented in Task 1 step 8c)
   - Load from fp32 dir

4. Run a full cargo build to verify everything links:
```bash
cd src-tauri && cargo build --features whisper,parakeet 2>&1 | tail -20
```
  </action>
  <verify>
    <automated>cd src-tauri && cargo build --features whisper,parakeet 2>&1 | tail -10</automated>
  </verify>
  <done>
    - set_engine accepts optional parakeet_model parameter
    - Existing callers without parakeet_model parameter still work (None default)
    - parakeet_model is persisted to settings.json when provided
    - Full cargo build succeeds with both features
    - Variant switching reloads model from correct directory
  </done>
</task>

</tasks>

<verification>
1. `cargo check --features whisper,parakeet` passes — all Rust code compiles
2. `npx tsc --noEmit` passes — all TypeScript compiles
3. `list_models` returns 4 entries: large-v3-turbo, small-en, parakeet-tdt-v2, parakeet-tdt-v2-fp32
4. `download_parakeet_fp32_model` command is registered and callable
5. `set_engine` with parakeetModel parameter persists variant and loads correct model dir
</verification>

<success_criteria>
- Two Parakeet entries visible in settings model list and first-run
- fp32 download triggers separate download to models/parakeet-tdt-v2-fp32/
- Selecting fp32 sets engine to parakeet and loads from fp32 directory
- Selecting int8 sets engine to parakeet and loads from int8 directory
- Both can be downloaded independently (separate directories)
- Full build succeeds
</success_criteria>

<output>
After completion, create `.planning/quick/13-add-fp32-parakeet-model-variant-as-selec/13-SUMMARY.md`
</output>
