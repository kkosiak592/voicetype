---
status: awaiting_human_verify
trigger: "cuda-dlls-missing-after-install"
created: 2026-03-05T00:00:00Z
updated: 2026-03-05T00:00:00Z
---

## Current Focus

hypothesis: CONFIRMED — TAURI_CONFIG env var is silently ignored by tauri-action/Tauri v2 CLI. The installer is only 129MB (vs expected ~450-500MB with three bundled CUDA DLLs), proving the DLLs were never added to the installer. The correct mechanism is the tauri-action `args` input with `--config '{"bundle":{"resources":{...}}}'`.
test: Installer size (129MB vs 66MB v1.1.0 = only +63MB) conclusively proves DLLs absent from installer. cublasLt64 alone is ~530MB uncompressed, would produce 400-500MB compressed installer.
expecting: Fix by changing TAURI_CONFIG env var to args: --config JSON in the tauri-action step
next_action: Apply fix to release.yml — replace TAURI_CONFIG env var with args input on tauri-action step

## Symptoms

expected: cudart64_12.dll, cublas64_12.dll, cublasLt64_12.dll should appear next to VoiceType.exe in AppData\Local\VoiceType\ after installing v1.2.0-rc1
actual: Only benchmark.exe, uninstall.exe, voice-to-text.exe in the install directory. No DLLs visible.
errors: None — installer runs fine, app works, just no CUDA DLLs in the directory
reproduction: Download v1.2.0-rc1 installer from GitHub releases, install it, check AppData\Local\VoiceType\
started: First time testing CUDA DLL bundling

## Eliminated

- hypothesis: DLLs are in the installer but placed in a resources/ subdirectory rather than $INSTDIR root
  evidence: Installer is only 129MB vs 66MB for v1.1.0. cublasLt64_12.dll alone is ~530MB uncompressed. Even with LZMA compression, three DLLs would produce a 450-500MB installer. A 63MB delta is impossible if the DLLs were bundled anywhere in the installer.
  timestamp: 2026-03-05

## Evidence

- timestamp: 2026-03-05
  checked: release.yml TAURI_CONFIG env var on tauri-action step
  found: TAURI_CONFIG is set as env var on the tauri-apps/tauri-action@v0 step. The value is a JSON string with bundle.resources map.
  implication: tauri-action env vars are available to the step, but only if the action or the CLI explicitly reads them.

- timestamp: 2026-03-05
  checked: Tauri v2 official environment variables reference (v2.tauri.app/reference/environment-variables/)
  found: TAURI_CONFIG is NOT listed in the official Tauri v2 environment variables reference. The listed vars are: TAURI_CLI_CONFIG_DEPTH, TAURI_CLI_PORT, TAURI_CLI_WATCHER_IGNORE_FILENAME, TAURI_ENV_DEBUG, TAURI_ENV_TARGET_TRIPLE, TAURI_ENV_ARCH, TAURI_ENV_PLATFORM, TAURI_ENV_PLATFORM_VERSION, TAURI_ENV_FAMILY, TAURI_ENV_PLATFORM_TYPE, TAURI_WEBVIEW_AUTOMATION, TAURI_ANDROID_PROJECT_PATH, TAURI_DEV_HOST
  implication: TAURI_CONFIG is not a documented input env var for the Tauri v2 CLI. Setting it externally has no documented effect.

- timestamp: 2026-03-05
  checked: tauri-action documentation for passing config to tauri build
  found: The documented method for passing config overrides is the `args` input parameter with `--config JSON_OR_PATH`. There is no documented support for TAURI_CONFIG env var as external input in tauri-action.
  implication: The release.yml is using the wrong mechanism. The TAURI_CONFIG env var is silently ignored by the Tauri CLI.

- timestamp: 2026-03-05
  checked: Installer size for v1.2.0-rc1 (129MB) vs v1.1.0 (66MB)
  found: The increase is only 63MB. Three CUDA DLLs (cublasLt64 ~530MB, cublas ~75MB, cudart ~3MB = ~608MB total uncompressed) would produce an installer of ~450-500MB even with LZMA compression.
  implication: The DLLs were never included in the installer. The TAURI_CONFIG env var was indeed silently ignored.

- timestamp: 2026-03-05
  checked: TAURI_CONFIG role in Tauri internals (Fossies source reference)
  found: TAURI_CONFIG appears to be an INTERNAL env var that the Tauri CLI sets when passing a config patch from CLI layer to the Rust core (build.rs reads it). It is not a user-facing input — it's a communication channel within the Tauri build pipeline.
  implication: Setting TAURI_CONFIG externally from the GitHub Actions env block is overwriting the Tauri CLI's internal channel, but the CLI itself is NOT reading it from the environment as user input — the CLI reads config only from --config flag or from the static config files.

## Resolution

root_cause: TAURI_CONFIG is not a user-facing Tauri v2 CLI input environment variable. It is an internal env var that the Tauri build pipeline sets when passing a config patch from the CLI layer to the Rust core (read by tauri-build's build.rs). Setting it externally on the tauri-action step env block has no effect on config merging — the Tauri CLI reads config overrides only from the --config flag. Because TAURI_CONFIG was silently ignored, the CUDA DLLs were never included in the NSIS installer despite being correctly staged in src-tauri/cuda-libs/.
fix: Removed TAURI_CONFIG from the env: block on the tauri-action step. Added args: --config '<json>' to the with: block, which passes the config override via the --config CLI flag through tauri-action's documented args input parameter.
verification: Requires a CI run. Expected result: installer size increases from ~129MB to ~450-500MB (dominated by cublasLt64_12.dll). Post-install directory check should show cudart64_12.dll, cublas64_12.dll, cublasLt64_12.dll alongside VoiceType.exe in AppData\Local\VoiceType\.
files_changed:
  - .github/workflows/release.yml

root_cause:
fix:
verification:
files_changed: []
