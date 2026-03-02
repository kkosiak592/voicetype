---
phase: quick-10
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/Cargo.toml
  - src-tauri/src/transcribe_parakeet.rs
  - src-tauri/src/lib.rs
autonomous: true
requirements: []
must_haves:
  truths:
    - "Parakeet TDT model loads with CUDA ExecutionProvider when use_cuda=true"
    - "ONNX Runtime uses GPU for Parakeet inference instead of CPU"
    - "Build compiles with cuda feature enabled on parakeet-rs"
  artifacts:
    - path: "src-tauri/Cargo.toml"
      provides: "parakeet-rs dependency with cuda feature enabled"
      contains: 'features = ["cuda"]'
    - path: "src-tauri/src/transcribe_parakeet.rs"
      provides: "CUDA ExecutionProvider config passed to from_pretrained"
      contains: "ExecutionProvider::Cuda"
  key_links:
    - from: "src-tauri/src/transcribe_parakeet.rs"
      to: "parakeet-rs execution::ModelConfig"
      via: "ModelConfig with ExecutionProvider::Cuda"
      pattern: "ExecutionConfig.*with_execution_provider.*Cuda"
---

<objective>
Enable CUDA GPU acceleration for Parakeet ONNX inference.

Purpose: Parakeet currently runs on CPU ExecutionProvider. The CUDA toolkit is already installed (used by whisper-rs), and ort downloads pre-compiled CUDA binaries via download-binaries feature. Enabling the cuda feature on parakeet-rs and passing CUDA ExecutionProvider config will route inference through GPU for significant speedup.

Output: Parakeet loads and runs on CUDA EP; falls back to CPU if CUDA unavailable (ort built-in fallback behavior).
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src-tauri/Cargo.toml
@src-tauri/src/transcribe_parakeet.rs
@src-tauri/src/lib.rs
@src-tauri/patches/parakeet-rs/src/execution.rs

Key types from parakeet-rs patch (src-tauri/patches/parakeet-rs/src/execution.rs):
```rust
// When feature = "cuda" is enabled, ExecutionProvider::Cuda becomes available
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExecutionProvider {
    #[default]
    Cpu,
    #[cfg(feature = "cuda")]
    Cuda,
    // ...
}

pub struct ModelConfig {
    pub execution_provider: ExecutionProvider,
    pub intra_threads: usize,
    pub inter_threads: usize,
}
// ModelConfig::new().with_execution_provider(ExecutionProvider::Cuda)
```

From parakeet_tdt.rs:
```rust
pub fn from_pretrained<P: AsRef<Path>>(path: P, config: Option<ExecutionConfig>) -> Result<Self>
// ExecutionConfig is `use crate::execution::ModelConfig as ExecutionConfig`
```
</context>

<tasks>

<task type="auto">
  <name>Task 1: Enable cuda feature on parakeet-rs and wire CUDA ExecutionProvider</name>
  <files>src-tauri/Cargo.toml, src-tauri/src/transcribe_parakeet.rs, src-tauri/src/lib.rs</files>
  <action>
Three changes:

1. **src-tauri/Cargo.toml** — Add `features = ["cuda"]` to the patched parakeet-rs dependency. Change line 71 from:
   ```toml
   parakeet-rs = { version = "0.1.9", optional = true }
   ```
   to:
   ```toml
   parakeet-rs = { version = "0.1.9", features = ["cuda"], optional = true }
   ```
   Also update the NOTE comment above it (lines 68-70) to reflect that cuda IS now enabled. Remove the "Add cuda feature once deployment environment is confirmed" line since we are enabling it now.

2. **src-tauri/src/transcribe_parakeet.rs** — Make `load_parakeet` actually use the `use_cuda` parameter:
   - Add import: `use parakeet_rs::execution::{ModelConfig as ExecutionConfig, ExecutionProvider};`
     (Note: parakeet_tdt.rs uses `use crate::execution::ModelConfig as ExecutionConfig` internally, but from outside the crate the public path is `parakeet_rs::execution::*`)
   - Rename `_use_cuda` to `use_cuda` (remove underscore prefix)
   - Replace the `None` config in `from_pretrained` with:
     ```rust
     let config = if use_cuda {
         log::info!("Requesting CUDA ExecutionProvider for Parakeet TDT");
         Some(ExecutionConfig::new().with_execution_provider(ExecutionProvider::Cuda))
     } else {
         None // CPU ExecutionProvider (default)
     };
     ```
   - Pass `config` to `ParakeetTDT::from_pretrained(model_dir, config)`
   - Update the doc comment to say "Uses CUDA execution provider when use_cuda=true, CPU otherwise."
   - Remove the old comment about CUDA not being enabled.

3. **src-tauri/src/lib.rs** — Change both calls from `load_parakeet(&dir_str, false)` to `load_parakeet(&dir_str, true)` at:
   - Line ~258 (startup loading)
   - Line ~1246 (engine switch loading)

IMPORTANT: Do NOT change the parakeet feature flag definition in [features] — it should remain `parakeet = ["dep:parakeet-rs"]` without adding "parakeet-rs/cuda" there, because the cuda feature is set directly on the dependency. The feature flag `parakeet` gates whether parakeet-rs is compiled at all; the cuda feature on the dep controls GPU support within it.
  </action>
  <verify>
    <automated>cd src-tauri && cargo check --features parakeet 2>&1 | tail -20</automated>
  </verify>
  <done>
    - Cargo.toml has `features = ["cuda"]` on parakeet-rs dependency
    - `load_parakeet` constructs `ExecutionConfig` with `ExecutionProvider::Cuda` when `use_cuda=true`
    - Both call sites in lib.rs pass `true` for use_cuda
    - `cargo check` passes with no errors
  </done>
</task>

</tasks>

<verification>
- `cargo check --features parakeet` compiles without errors
- grep confirms `ExecutionProvider::Cuda` in transcribe_parakeet.rs
- grep confirms `load_parakeet(&dir_str, true)` in lib.rs (two occurrences)
- grep confirms `features = ["cuda"]` on parakeet-rs in Cargo.toml
</verification>

<success_criteria>
Parakeet ONNX inference configured to use CUDA GPU ExecutionProvider. Build compiles. At runtime, ort will use CUDA EP with automatic CPU fallback if CUDA is unavailable.
</success_criteria>

<output>
After completion, create `.planning/quick/10-enable-cuda-gpu-acceleration-for-parakee/10-SUMMARY.md`
</output>
