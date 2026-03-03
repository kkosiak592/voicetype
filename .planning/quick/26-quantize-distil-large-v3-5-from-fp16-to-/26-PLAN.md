---
phase: 26-quantize-distil-large-v3-5
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/src/download.rs
  - src-tauri/src/lib.rs
  - src/components/FirstRun.tsx
autonomous: false
requirements: [QUANT-01]

must_haves:
  truths:
    - "distil-large-v3.5 model downloads as q5_0 quantized (~600MB) instead of fp16 (1.52GB)"
    - "Downloaded q5_0 model passes SHA256 verification"
    - "FirstRun UI shows correct smaller file size for distil-large-v3.5"
    - "Transcription with quantized model works (model loads and produces output)"
  artifacts:
    - path: "src-tauri/src/download.rs"
      provides: "Updated URL, SHA256, and size for distil-large-v3.5 q5_0"
      contains: "q5_0"
    - path: "src-tauri/src/lib.rs"
      provides: "Updated model description with correct size"
    - path: "src/components/FirstRun.tsx"
      provides: "Updated model card size display"
  key_links:
    - from: "src-tauri/src/download.rs"
      to: "HuggingFace hosted q5_0 file"
      via: "model_info URL for distil-large-v3.5"
      pattern: "distil-large-v3\\.5.*q5_0"
    - from: "src/components/FirstRun.tsx"
      to: "src-tauri/src/download.rs"
      via: "size string must match actual file size"
---

<objective>
Replace the fp16 distil-large-v3.5 model with a q5_0 quantized version (~600MB instead of 1.52GB, 30-50% faster inference).

Purpose: The fp16 model at 1.52GB is unnecessarily large for CPU inference. Quantizing to q5_0 reduces download size by ~60% and improves inference speed with negligible quality loss — matching the quantization approach already used for large-v3-turbo (q5_0) and small-en (q5_1).

Output: Updated download.rs, lib.rs, and FirstRun.tsx pointing at hosted q5_0 model file.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src-tauri/src/download.rs
@src-tauri/src/lib.rs
@src/components/FirstRun.tsx

<interfaces>
<!-- From download.rs — the model_info entry to update (lines 69-74): -->
```rust
"distil-large-v3.5" => Some((
    "ggml-distil-large-v3.5.bin",
    "https://huggingface.co/distil-whisper/distil-large-v3.5-ggml/resolve/main/ggml-model.bin",
    "ec2498919b498c5f6b00041adb45650124b3cd9f26f545fffa8f5d11c28dcf26",
    1_519_521_155,
)),
```

<!-- From lib.rs — the model description to update (line 1105): -->
```rust
description: "High accuracy — 1.52 GB — GPU accelerated when available".to_string(),
```

<!-- From FirstRun.tsx — the MODELS array entry to update (lines 30-36): -->
```typescript
{
  id: 'distil-large-v3.5',
  name: 'Distil Large v3.5',
  size: '1.52 GB',
  quality: 'High accuracy, fast',
  requirement: 'GPU recommended, works on any hardware',
  gpuOnly: false,
},
```

<!-- From lib.rs — filename references (lines 1055, 1107, 1211): -->
<!-- The local filename "ggml-distil-large-v3.5.bin" does NOT need to change. -->
<!-- Only the download URL, SHA256, and size change. -->
</interfaces>
</context>

<tasks>

<task type="checkpoint:human-action" gate="blocking">
  <name>Task 1: Quantize fp16 model to q5_0 and host</name>
  <files>none (infrastructure step — produces hosted model file)</files>
  <action>
The user must produce a q5_0 quantized GGML file from the fp16 distil-large-v3.5 model. This cannot be automated by Claude because it requires: (a) downloading a 1.52GB file, (b) building the whisper.cpp quantize tool from source, (c) running the quantize binary locally, and (d) uploading the result to a HuggingFace repository.

Steps for the user:

1. **Clone whisper.cpp and build the quantize tool:**
   ```bash
   git clone https://github.com/ggerganov/whisper.cpp.git
   cd whisper.cpp
   cmake -B build
   cmake --build build --config Release
   # Binary will be at: build/bin/Release/quantize.exe (Windows) or build/bin/quantize (Linux/Mac)
   ```

2. **Download the fp16 model:**
   ```bash
   curl -L -o ggml-distil-large-v3.5-fp16.bin "https://huggingface.co/distil-whisper/distil-large-v3.5-ggml/resolve/main/ggml-model.bin"
   ```
   Verify SHA256 matches: `ec2498919b498c5f6b00041adb45650124b3cd9f26f545fffa8f5d11c28dcf26`

3. **Quantize to q5_0:**
   ```bash
   ./build/bin/quantize ggml-distil-large-v3.5-fp16.bin ggml-distil-large-v3.5-q5_0.bin q5_0
   ```

4. **Compute SHA256 and file size of the output:**
   ```bash
   # Linux/Mac:
   sha256sum ggml-distil-large-v3.5-q5_0.bin
   wc -c ggml-distil-large-v3.5-q5_0.bin
   # Windows (PowerShell):
   Get-FileHash ggml-distil-large-v3.5-q5_0.bin -Algorithm SHA256
   (Get-Item ggml-distil-large-v3.5-q5_0.bin).Length
   ```

5. **Upload to HuggingFace** (e.g., to user's own repo or as a PR to `distil-whisper/distil-large-v3.5-ggml`):
   ```bash
   huggingface-cli upload <your-repo> ggml-distil-large-v3.5-q5_0.bin
   ```

6. **Report back to Claude with:**
   - The download URL (e.g., `https://huggingface.co/<repo>/resolve/main/ggml-distil-large-v3.5-q5_0.bin`)
   - The SHA256 hex string
   - The file size in bytes

Resume signal: Provide (1) download URL, (2) SHA256 hash, (3) file size in bytes of the q5_0 model.
  </action>
  <verify>User provides URL, SHA256, and file size — all three values are non-empty</verify>
  <done>q5_0 quantized model file is hosted at a public URL with known SHA256 and byte size</done>
</task>

<task type="auto">
  <name>Task 2: Update download.rs, lib.rs, and FirstRun.tsx with q5_0 model metadata</name>
  <files>src-tauri/src/download.rs, src-tauri/src/lib.rs, src/components/FirstRun.tsx</files>
  <action>
Using the URL, SHA256, and size provided by the user in Task 1, update these three files:

**1. src-tauri/src/download.rs** — In `model_info()`, update the `"distil-large-v3.5"` match arm (lines 69-74):
- Keep filename as `"ggml-distil-large-v3.5.bin"` (no change — this is the local save name)
- Replace URL with the user-provided q5_0 URL
- Replace SHA256 with the user-provided hash
- Replace `1_519_521_155` with the user-provided size in bytes
- Add a comment noting this is q5_0 quantized (not fp16)

**2. src-tauri/src/lib.rs** — Update the `ModelInfo` description (line 1105):
- Change `"High accuracy — 1.52 GB — GPU accelerated when available"` to use the correct size
- Compute display size: divide bytes by 1,073,741,824 for GB or 1,048,576 for MB. If under 1 GB, show MB. Format to match existing patterns (e.g., "574 MB", "190 MB", "1.52 GB").

**3. src/components/FirstRun.tsx** — In the MODELS array (line 32):
- Change `size: '1.52 GB'` to the correct human-readable size matching the lib.rs description

Ensure all three locations show consistent size values.
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text &amp;&amp; cargo check --manifest-path src-tauri/Cargo.toml 2>&amp;1 | tail -5 &amp;&amp; npx tsc --noEmit 2>&amp;1 | tail -5</automated>
  </verify>
  <done>
- download.rs model_info for "distil-large-v3.5" returns q5_0 URL, correct SHA256, and correct byte size
- lib.rs ModelInfo description shows correct smaller size
- FirstRun.tsx MODELS array shows correct smaller size
- cargo check passes (no Rust compilation errors)
- TypeScript type-check passes (no TS errors)
  </done>
</task>

<task type="checkpoint:human-verify" gate="blocking">
  <name>Task 3: Verify q5_0 download and transcription</name>
  <files>none (verification only)</files>
  <action>
What was built: Updated distil-large-v3.5 from fp16 (1.52 GB) to q5_0 (~600 MB) across download.rs, lib.rs, and FirstRun.tsx.

How to verify:
1. Run `cargo build --manifest-path src-tauri/Cargo.toml` — should compile without errors
2. Delete existing `ggml-distil-large-v3.5.bin` from `%APPDATA%/VoiceType/models/` (if present)
3. Launch the app with `npm run tauri dev`
4. On the FirstRun screen, verify the distil-large-v3.5 card shows the smaller size (not "1.52 GB")
5. Click Download on distil-large-v3.5 — verify it downloads ~600MB (not 1.52GB)
6. After download completes, verify transcription works with the quantized model

Resume signal: Type "approved" if download and transcription work, or describe issues.
  </action>
  <verify>User confirms download completes at smaller size and transcription produces output</verify>
  <done>q5_0 model downloads, passes SHA256 verification, and produces correct transcription output</done>
</task>

</tasks>

<verification>
- `cargo check` passes with updated download.rs constants
- TypeScript compiles with updated FirstRun.tsx size string
- All three files (download.rs, lib.rs, FirstRun.tsx) show consistent size for distil-large-v3.5
- The q5_0 model downloads successfully and passes SHA256 verification
- Transcription with q5_0 model produces correct output
</verification>

<success_criteria>
- distil-large-v3.5 downloads as q5_0 quantized file (~600MB) instead of fp16 (1.52GB)
- SHA256 verification passes on downloaded file
- UI displays correct file size
- Transcription quality is acceptable with quantized model
</success_criteria>

<output>
After completion, create `.planning/quick/26-quantize-distil-large-v3-5-from-fp16-to-/26-SUMMARY.md`
</output>
