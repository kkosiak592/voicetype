---
status: awaiting_human_verify
trigger: "Parakeet TDT ONNX inference crashes with indices element out of data bounds idx=8192 at /decoder/embed/Gather"
created: 2026-03-01T00:00:00Z
updated: 2026-03-01T00:10:00Z
---

## Current Focus

hypothesis: CONFIRMED. parakeet-rs 0.1.9 hardcodes vocab_size=8193, computing blank_id=8192 which is OOB for the int8 model's Gather table (size 1025).
test: Applied Cargo patch; ran cargo check --features parakeet → compiled cleanly.
expecting: With vocab_size=1025 read from config.json → blank_id=1024. Transcription completes without OOB Gather error.
next_action: User verifies end-to-end transcription works with Parakeet engine selected.

## Symptoms

expected: Parakeet TDT transcribes speech and returns text
actual: ONNX Runtime crashes at Gather node with index 8192 out of bounds [-1025,1024]
errors: "ONNX Runtime error: Non-zero status code returned while running Gather node. Name:'/decoder/embed/Gather' Status Message: indices element out of data bounds, idx=8192 must be within the inclusive range [-1025,1024]"
reproduction: Select Parakeet engine, press shortcut key, speak, release. Every transcription attempt fails with this error.
started: First time testing Parakeet inference after fixing model download and feature flag issues.

## Eliminated

- hypothesis: Bug in our application code (transcribe_parakeet.rs)
  evidence: transcribe_parakeet.rs simply calls ParakeetTDT::from_pretrained and transcribe_samples with no vocab config. The bug is entirely inside parakeet-rs model_tdt.rs line 16 (vocab_size: 8193) and line 159 (blank_id = vocab_size - 1 = 8192).
  timestamp: 2026-03-01T00:00:00Z

- hypothesis: Model's config.json is not parsed by from_pretrained at all
  evidence: Confirmed. parakeet_tdt.rs::from_pretrained calls ParakeetTDTModel::from_pretrained which does TDTModelConfig::default() — never reads config.json. ModelConfig struct in config.rs has vocab_size field and a correct default of 1025, but that struct is never used by the TDT path.
  timestamp: 2026-03-01T00:00:00Z

## Evidence

- timestamp: 2026-03-01T00:00:00Z
  checked: parakeet-rs-0.1.9/src/model_tdt.rs lines 13-19, 39, 159
  found: TDTModelConfig::default() hardcodes vocab_size=8193. from_pretrained uses TDTModelConfig::default() without reading config.json. blank_id = vocab_size - 1 = 8192, passed as last_emitted_token to the decoder_joint Gather node.
  implication: Every inference call passes idx=8192 to a Gather node whose table only holds 1025 entries (0..1024), causing the OOB crash.

- timestamp: 2026-03-01T00:00:00Z
  checked: parakeet-rs-0.1.9/src/config.rs lines 18-50
  found: ModelConfig struct has vocab_size: usize and pad_token_id: usize. Default impl sets vocab_size=1025 and pad_token_id=1024 — exactly correct for the int8 model. serde_json is available in parakeet-rs (in Cargo.toml dependencies).
  implication: We can read config.json using existing serde + serde_json infrastructure already in the crate. No new dependencies needed in the patch.

- timestamp: 2026-03-01T00:00:00Z
  checked: parakeet-rs-0.1.9/src/parakeet_tdt.rs (from_pretrained)
  found: from_pretrained looks for vocab.txt but never reads config.json. It calls ParakeetTDTModel::from_pretrained(path, exec_config) passing no vocab info.
  implication: The fix can be applied entirely in model_tdt.rs::from_pretrained: read config.json, use its vocab_size to populate TDTModelConfig instead of the hardcoded default.

- timestamp: 2026-03-01T00:00:00Z
  checked: src-tauri/patches/esaxx-rs/ (existing patch pattern)
  found: Local crate patch is a full copy of the crate source under patches/parakeet-rs/ referenced via [patch.crates-io] in Cargo.toml. The esaxx-rs patch works by pointing at the local directory.
  implication: Same pattern applies: copy parakeet-rs-0.1.9 source to patches/parakeet-rs/, edit model_tdt.rs, add [patch.crates-io] entry.

## Resolution

root_cause: parakeet-rs 0.1.9 TDTModelConfig::default() hardcodes vocab_size=8193 (designed for the full parakeet-tdt-0.6b model), but the actual downloaded model (parakeet-tdt-0.6b-v2 int8) has vocab_size=1025. The greedy decoder computes blank_id=8192, which is passed to the decoder_joint ONNX Gather node that only has 1025 entries (valid indices 0..1024), causing an OOB crash on every inference call.
fix: Created local Cargo patch of parakeet-rs-0.1.9 under src-tauri/patches/parakeet-rs/. Modified model_tdt.rs to add load_config() that reads config.json via serde_json (already a crate dependency) and populates TDTModelConfig.vocab_size from it. Added [patch.crates-io] entry in src-tauri/Cargo.toml. cargo check --features parakeet passes cleanly.
verification: cargo check passes. End-to-end runtime verification pending user confirmation.
files_changed:
  - src-tauri/patches/parakeet-rs/src/model_tdt.rs (added load_config(), import of ModelConfigJson, replaced TDTModelConfig::default() call with Self::load_config(model_dir))
  - src-tauri/Cargo.toml (added parakeet-rs = { path = "patches/parakeet-rs" } under [patch.crates-io])
