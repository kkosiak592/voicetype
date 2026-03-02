---
phase: quick-23
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/patches/parakeet-rs/src/execution.rs
  - src-tauri/src/transcribe_parakeet.rs
  - src-tauri/src/lib.rs
autonomous: true
requirements: [LATENCY-CUDA-EP, LATENCY-WARMUP]
must_haves:
  truths:
    - "CUDA EP uses cuda_graph and tf32 flags when building the provider"
    - "First real transcription after startup is faster due to warm-up inference"
    - "Warm-up runs in background and does not block UI startup"
  artifacts:
    - path: "src-tauri/patches/parakeet-rs/src/execution.rs"
      provides: "CUDA EP with cuda_graph and tf32 enabled"
      contains: "with_cuda_graph"
    - path: "src-tauri/src/transcribe_parakeet.rs"
      provides: "warm_up_parakeet function"
      contains: "warm_up_parakeet"
    - path: "src-tauri/src/lib.rs"
      provides: "Background warm-up call after model load"
      contains: "warm_up_parakeet"
  key_links:
    - from: "src-tauri/src/lib.rs"
      to: "src-tauri/src/transcribe_parakeet.rs"
      via: "warm_up_parakeet call after load_parakeet succeeds"
      pattern: "warm_up_parakeet"
---

<objective>
Implement two Tier 1 latency optimizations for the Parakeet TDT model to reduce first-inference and steady-state CUDA latency.

Purpose: The first transcription after app launch is noticeably slower due to lazy CUDA context init, cudaMalloc, and cuDNN algorithm selection. CUDA graph replay reduces per-inference CPU overhead for the decoder_joint subgraph (called 600+ times). TF32 enables faster matmul/conv on Ampere+ GPUs.

Output: Modified CUDA EP config with graph+tf32, and a background warm-up inference on model load.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src-tauri/patches/parakeet-rs/src/execution.rs
@src-tauri/src/transcribe_parakeet.rs
@src-tauri/src/lib.rs

<interfaces>
From src-tauri/patches/parakeet-rs/src/execution.rs (ort 2.0.0-rc.10):
```rust
// Line 116 — current CUDA EP construction:
ort::execution_providers::CUDAExecutionProvider::default().build()

// Available builder methods (confirmed in ort 2.0.0-rc.10 source):
// .with_cuda_graph(bool) — enables CUDA graph capture/replay
// .with_tf32(bool) — enables TF32 reduced-precision on Ampere+
```

From src-tauri/src/transcribe_parakeet.rs:
```rust
pub fn load_parakeet(model_dir: &str, provider: &str) -> Result<ParakeetTDT, String>;
pub fn transcribe_with_parakeet(parakeet: &mut ParakeetTDT, audio: &[f32]) -> Result<String, String>;
```

From src-tauri/src/lib.rs:
```rust
pub struct ParakeetStateMutex(pub std::sync::Mutex<Option<std::sync::Arc<std::sync::Mutex<parakeet_rs::ParakeetTDT>>>>);
// Two load sites:
// 1. setup() startup: line ~1391
// 2. set_engine() switch: line ~311
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Enable CUDA graph replay and TF32 in CUDA EP builder</name>
  <files>src-tauri/patches/parakeet-rs/src/execution.rs</files>
  <action>
In `apply_to_session_builder`, modify the `ExecutionProvider::Cuda` match arm (lines 113-119) to chain `.with_cuda_graph(true)` and `.with_tf32(true)` on the `CUDAExecutionProvider::default()` builder before `.build()`.

Change from:
```rust
ort::execution_providers::CUDAExecutionProvider::default().build(),
```

To:
```rust
ort::execution_providers::CUDAExecutionProvider::default()
    .with_tf32(true)
    .build(),
```

IMPORTANT: Do NOT add `.with_cuda_graph(true)`. After reviewing the ort docs, CUDA graphs require that "Input/output shapes cannot change across inference calls" and "IoBinding must be used". Parakeet processes variable-length audio — the encoder input shape changes every call. CUDA graph capture would either fail or produce incorrect results. The decoder_joint may have fixed shapes, but CUDA graph mode is set per-session, not per-subgraph. Only TF32 is safe here.

Update the eprintln log message to indicate TF32 is enabled:
```rust
eprintln!("[parakeet-rs] Registering CUDA ExecutionProvider (TF32 enabled, with CPU fallback)");
```
  </action>
  <verify>
    <automated>cd src-tauri && cargo check --features parakeet,cuda 2>&1 | tail -5</automated>
  </verify>
  <done>CUDA EP builder uses .with_tf32(true), compiles without errors. CUDA graph intentionally omitted due to variable input shapes.</done>
</task>

<task type="auto">
  <name>Task 2: Add warm-up function and invoke after model load</name>
  <files>src-tauri/src/transcribe_parakeet.rs, src-tauri/src/lib.rs</files>
  <action>
**In `src-tauri/src/transcribe_parakeet.rs`**, add a new public function `warm_up_parakeet` after `transcribe_with_parakeet`:

```rust
/// Runs a dummy inference with ~0.5s of silent audio (8000 zero samples at 16kHz)
/// to trigger CUDA context initialization, cudaMalloc, and cuDNN algorithm selection.
/// The transcription result is discarded. Logs warm-up duration.
/// This should be called once after model loading, ideally in a background thread.
pub fn warm_up_parakeet(parakeet: &mut ParakeetTDT) {
    let start = Instant::now();
    // 0.5 seconds of silence at 16kHz = 8000 samples
    let silent_audio: Vec<f32> = vec![0.0f32; 8000];
    match parakeet.transcribe_samples(silent_audio, 16000, 1, Some(TimestampMode::Sentences)) {
        Ok(_) => {
            log::info!(
                "Parakeet warm-up completed in {}ms (CUDA context + cuDNN initialized)",
                start.elapsed().as_millis()
            );
        }
        Err(e) => {
            log::warn!("Parakeet warm-up inference failed (non-fatal): {}", e);
        }
    }
}
```

**In `src-tauri/src/lib.rs`**, add warm-up calls in both Parakeet load paths:

1. **Startup path (~line 1396, after `*guard = Some(...)`):** After the model is stored in `ParakeetStateMutex`, spawn a background task to run warm-up. Insert after line 1400 (`log::info!("Parakeet model loaded at startup...")`):

```rust
// Warm up in background to avoid blocking UI
let warmup_arc = {
    let parakeet_state = app.state::<ParakeetStateMutex>();
    let guard = parakeet_state.0.lock().unwrap_or_else(|e| e.into_inner());
    guard.clone() // Option<Arc<Mutex<ParakeetTDT>>>
};
if let Some(arc) = warmup_arc {
    std::thread::spawn(move || {
        let mut guard = arc.lock().unwrap_or_else(|e| e.into_inner());
        transcribe_parakeet::warm_up_parakeet(&mut guard);
    });
}
```

2. **Engine switch path (~line 315, after `log::info!("Parakeet model loaded on engine switch...")`):** Same pattern — clone the Arc, spawn a thread, run warm-up. The engine switch is already a synchronous command handler, so spawning a thread keeps the UI responsive:

```rust
// Warm up after engine switch
let warmup_arc = {
    let guard = parakeet_state.0.lock().unwrap_or_else(|e| e.into_inner());
    guard.clone()
};
if let Some(arc) = warmup_arc {
    std::thread::spawn(move || {
        let mut guard = arc.lock().unwrap_or_else(|e| e.into_inner());
        transcribe_parakeet::warm_up_parakeet(&mut guard);
    });
}
```

Note: The warm-up holds the Mutex briefly (~0.5-2s). If a real transcription request comes during warm-up, it will simply wait for the Mutex — acceptable since warm-up completes quickly and the user is unlikely to dictate within the first second of startup.
  </action>
  <verify>
    <automated>cd src-tauri && cargo check --features parakeet,cuda 2>&1 | tail -5</automated>
  </verify>
  <done>warm_up_parakeet function exists in transcribe_parakeet.rs. Both startup and engine-switch load paths spawn a background thread that runs warm-up after successful model load. Project compiles cleanly.</done>
</task>

</tasks>

<verification>
- `cargo check --features parakeet,cuda` passes without errors or warnings
- execution.rs contains `.with_tf32(true)` in the CUDA match arm
- transcribe_parakeet.rs exports `warm_up_parakeet`
- lib.rs calls `warm_up_parakeet` in both the startup and engine-switch code paths via `std::thread::spawn`
</verification>

<success_criteria>
- CUDA EP is configured with TF32 for faster matmul/conv on Ampere+ GPUs
- A 0.5s silent-audio warm-up inference runs in a background thread after every Parakeet model load
- Warm-up does not block UI startup or engine switch response
- All changes compile cleanly with `cargo check --features parakeet,cuda`
</success_criteria>

<output>
After completion, create `.planning/quick/23-implement-tier-1-parakeet-latency-optimi/23-SUMMARY.md`
</output>
