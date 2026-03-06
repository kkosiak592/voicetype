---
status: verifying
trigger: "App crashes on startup with nvcuda.dll was not found on non-NVIDIA machine"
created: 2026-03-05T00:00:00Z
updated: 2026-03-05T00:00:00Z
---

## Current Focus

hypothesis: CONFIRMED. whisper-rs-sys with cuda feature calls cargo:rustc-link-lib=cuda which links against cuda.lib → creates hard load-time import of nvcuda.dll in voice_to_text_lib.dll and voice-to-text.exe. This fails at process startup on non-NVIDIA machines.
test: Built with /DELAYLOAD:nvcuda.dll in build.rs. Verified PE import tables: nvcuda.dll moved from regular imports to delay-import section in both voice_to_text_lib.dll and voice-to-text.exe.
expecting: Non-NVIDIA machines can start the app because nvcuda.dll is never loaded (CUDA code path never reached after NVML detection).
next_action: await human verification that fix works on non-NVIDIA machine

## Symptoms

expected: App should start on any machine — use CUDA on NVIDIA, DirectML on AMD/Intel discrete, CPU otherwise. Graceful runtime fallback.
actual: "The code execution cannot proceed because nvcuda.dll was not found" system error dialog on a non-NVIDIA computer. App won't start at all.
errors: Windows System Error: "voice-to-text.exe - System Error: The code execution cannot proceed because nvcuda.dll was not found."
reproduction: Install v1.2.0-rc2 on a machine without NVIDIA GPU/drivers. Launch the app.
started: Started with v1.2.0-rc2 which bundles CUDA redistributable DLLs next to the exe. v1.1.0 presumably worked fine.

## Eliminated

(none yet)

## Evidence

- timestamp: 2026-03-05T00:00:00Z
  checked: ort-sys-2.0.0-rc.10 build.rs lines 522-534
  found: When any of cuda or tensorrt features are enabled, ort-sys selects feature_set "cu12" and downloads the cu12 ONNX Runtime binary package (x86_64-pc-windows-msvc+cu12.tgz from cdn.pyke.io)
  implication: The cu12 binary includes onnxruntime_providers_cuda.dll and onnxruntime_providers_tensorrt.dll which have a hard load-time dependency on nvcuda.dll

- timestamp: 2026-03-05T00:00:00Z
  checked: target/release/*.dll
  found: Release build output contains onnxruntime_providers_cuda.dll, onnxruntime_providers_tensorrt.dll, onnxruntime_providers_shared.dll, DirectML.dll at the release root (copied by ort-sys copy-dylibs feature)
  implication: Tauri bundles all DLLs from the binary directory next to the exe in the installer. When Windows loads the exe, it resolves all DLL imports transitively at startup — including nvcuda.dll which onnxruntime_providers_cuda.dll imports.

- timestamp: 2026-03-05T00:00:00Z
  checked: src-tauri/Cargo.toml features for parakeet-rs and transcribe-rs
  found: parakeet-rs = { version = "0.1.9", features = ["cuda", "directml"], ... }; transcribe-rs = { version = "0.2.8", features = ["moonshine", "sense_voice", "cuda", "directml"], ... }
  implication: Both enable ort/cuda which triggers the cu12 binary download in ort-sys. The moonshine feature of transcribe-rs is in the main binary (not benchmark-only). Even if we use load-dynamic on the ort dep directly, these transitive features cause ort-sys to pick the cu12 binary.

- timestamp: 2026-03-05T00:00:00Z
  checked: ort Cargo.toml features list
  found: ort has a "load-dynamic" feature (uses libloading to load onnxruntime.dll at runtime). When load-dynamic is enabled, ort-sys still downloads the cu12 binary but does NOT statically link to it — it loads via LoadLibraryW at runtime. The CUDA/TensorRT provider DLLs (onnxruntime_providers_cuda.dll, onnxruntime_providers_tensorrt.dll) are SEPARATE from the main onnxruntime.dll and are loaded lazily by onnxruntime.dll only when explicitly requested.
  implication: load-dynamic alone does NOT solve the problem because the CUDA provider DLLs are still copied next to the exe by copy-dylibs, and they still have load-time dependencies on nvcuda.dll.

- timestamp: 2026-03-05T00:00:00Z
  checked: ort-sys copy-dylibs behavior (build.rs lines 592-596 and 99-141)
  found: The copy_libraries function copies ALL .dll files from the ort lib directory to the output directory (target/release). This includes onnxruntime_providers_cuda.dll and onnxruntime_providers_tensorrt.dll. Tauri then bundles everything in target/release next to the exe.
  implication: Even with load-dynamic, the CUDA provider DLLs end up bundled. The root issue is that these DLLs are in the installer at all.

- timestamp: 2026-03-05T00:00:00Z
  checked: dist.txt — what is in the "none" feature ONNX Runtime binary vs cu12
  found: "none" feature set binary exists for x86_64-pc-windows-msvc (x86_64-pc-windows-msvc.tgz). This is a plain ONNX Runtime build without CUDA/TensorRT provider DLLs.
  implication: If we use the "none" feature set binary, onnxruntime_providers_cuda.dll and onnxruntime_providers_tensorrt.dll are not present at all. However, CUDA EP would not be available (needed for GPU inference on NVIDIA).

- timestamp: 2026-03-05T00:00:00Z
  checked: whisper-rs-sys-0.14.1/build.rs lines 50-68
  found: With cuda feature, emits cargo:rustc-link-lib=cuda (plus cublas, cudart, cublasLt). The "cuda" lib name on Windows links against nvcuda.dll (CUDA driver DLL, NOT redistributable). This creates a hard load-time import in the final binary.
  implication: This is the true root cause. voice_to_text_lib.dll has nvcuda.dll, cublas64_12.dll, cudart64_12.dll in its regular import table. cublas/cudart are redistributable and bundled, but nvcuda.dll is the NVIDIA GPU driver — absent on non-NVIDIA machines.

- timestamp: 2026-03-05T00:00:00Z
  checked: voice_to_text_lib.dll PE import tables before fix
  found: Regular imports included: nvcuda.dll, cublas64_12.dll, cudart64_12.dll, directml.dll, dxcore.dll, dxgi.dll, d3d12.dll. No delay-import section.
  implication: Windows fails to load the process on non-NVIDIA machines because nvcuda.dll is in the regular import table (not delay-loaded).

- timestamp: 2026-03-05T00:00:00Z
  checked: onnxruntime_providers_cuda.dll bundling behavior
  found: onnxruntime_providers_cuda.dll also imports nvcuda.dll but it is NOT in the regular import table of voice-to-text.exe or voice_to_text_lib.dll. It is a runtime plugin loaded by onnxruntime.dll via LoadLibraryW only when CUDA EP is explicitly registered. Safe to bundle next to exe.
  implication: onnxruntime_providers_cuda.dll being next to the exe is NOT the cause. Only whisper-rs's direct linker dependency on cuda.lib causes the startup crash.

- timestamp: 2026-03-05T00:00:00Z
  checked: voice_to_text_lib.dll and voice-to-text.exe PE import tables AFTER fix (build.rs adding /DELAYLOAD:nvcuda.dll + delayimp.lib)
  found: Regular imports: kernel32, advapi32, cublas64_12.dll, cudart64_12.dll, directml.dll, etc. — nvcuda.dll gone. Delay imports: nvcuda.dll only. Delay import directory present (RVA non-zero, size=64).
  implication: Fix confirmed. nvcuda.dll is now delay-loaded. It will only be resolved when whisper's CUDA code actually calls a driver function — which only happens after NVML detection confirms NVIDIA GPU presence.

## Resolution

root_cause: whisper-rs-sys with the cuda feature emits cargo:rustc-link-lib=cuda in its build.rs, which links against the CUDA driver stub library (cuda.lib → nvcuda.dll). This creates a hard load-time import of nvcuda.dll in the final voice_to_text_lib.dll / voice-to-text.exe. Windows resolves all load-time DLL imports before executing a single instruction of the app. On non-NVIDIA machines, nvcuda.dll (the NVIDIA GPU driver DLL) is absent, so Windows displays "nvcuda.dll was not found" and refuses to launch the process. The bundled CUDA redistributables (cublas64_12.dll, cudart64_12.dll, cublasLt64_12.dll) were fine — only nvcuda.dll is non-redistributable.

fix: Added /DELAYLOAD:nvcuda.dll and delayimp.lib linker flags in src-tauri/build.rs (conditional on windows+msvc target). Delay-loading changes nvcuda.dll from a load-time dependency to a runtime dependency resolved only on first call to a CUDA driver function. Since the app uses NVML to detect NVIDIA GPU presence before entering any CUDA code path, nvcuda.dll is never loaded on non-NVIDIA machines.

verification: PE import tables confirmed — nvcuda.dll moved from regular import section to delay-import section in both voice_to_text_lib.dll and voice-to-text.exe. cublas64_12.dll and cudart64_12.dll remain as regular imports (they are bundled as redistributables). onnxruntime_providers_cuda.dll is not in any regular import table (it is a runtime plugin).
files_changed:
  - src-tauri/build.rs
