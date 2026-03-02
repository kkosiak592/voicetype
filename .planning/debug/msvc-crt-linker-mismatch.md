---
status: awaiting_human_verify
trigger: "MSVC linker CRT mismatch when building with both whisper and parakeet features — libcpmt.lib (static CRT) vs msvcprt.lib (dynamic CRT) cause LNK2005 multiply defined symbols and fatal LNK1169."
created: 2026-03-01T19:00:00Z
updated: 2026-03-01T20:10:00Z
---

## Current Focus

hypothesis: CONFIRMED -- esaxx-rs 0.1.10 hardcodes .static_crt(true) in build.rs, which compiles esaxx.cpp with /MT (MD_StaticRelease). onnxruntime.lib (from ort-sys) is compiled /MD (MD_DynamicRelease). MSVC LNK2038 treats this as a fatal mismatch that cannot be suppressed.
test: Applied [patch.crates-io] in Cargo.toml pointing to patches/esaxx-rs/ with .static_crt(false). Ran release build with STATIC_VCRUNTIME=true and both whisper+parakeet features.
expecting: No linker errors.
next_action: human verification — run npx tauri build and confirm it produces a working binary

## Symptoms

expected: npx tauri dev --features whisper compiles and links successfully
actual: Linker fails with ~18 LNK2005 errors (multiply defined symbols between msvcprt.lib and libcpmt.lib) plus fatal LNK1169. Warning LNK4098 says "defaultlib 'LIBCMT' conflicts with use of other libs"
errors: msvcprt.lib(MSVCP140.dll) : error LNK2005 multiply defined in libcpmt.lib. LINK warning LNK4098 defaultlib LIBCMT conflicts. fatal error LNK1169 one or more multiply defined symbols found.
reproduction: Run npx tauri build (release) with both whisper and parakeet features active. npx tauri build sets STATIC_VCRUNTIME=true which triggers the static CRT forcing in tauri-build.
started: After adding parakeet-rs dependency in phase 08. Building with whisper feature alone worked before.

## Eliminated

- hypothesis: whisper-rs-sys uses /MT (static CRT) compilation
  evidence: dumpbin /directives shows ggml-base.lib, ggml-cuda.lib etc. all have RuntimeLibrary=MD_DynamicRelease. The cmake build uses /MD flag throughout (confirmed in build output at target/debug/build/whisper-rs-sys-*/output).
  timestamp: 2026-03-01T19:05:00Z

- hypothesis: CUDA static runtime (cudart static) injects LIBCMT
  evidence: The build uses cargo:rustc-link-lib=cudart (dynamic import), not cudart_static.lib. nvcc compilation uses -Xcompiler "/MD" flag. No libcpmt or /MT flags found in whisper build output.
  timestamp: 2026-03-01T19:08:00Z

- hypothesis: ort-sys downloads a static CRT onnxruntime.lib
  evidence: dumpbin /directives on onnxruntime.lib (303MB) shows RuntimeLibrary=MD_DynamicRelease and /DEFAULTLIB:msvcprt. The ORT library was compiled /MD (dynamic CRT).
  timestamp: 2026-03-01T19:10:00Z

- hypothesis: Fix is NODEFAULTLIB:libcpmt.lib in build.rs (first attempted fix)
  evidence: User reported new error: LNK2038 RuntimeLibrary mismatch (850 mismatches). LNK2038 is a hard FAILIFMISMATCH directive — cannot be suppressed with /NODEFAULTLIB. First fix treated the symptom, not the root cause.
  timestamp: 2026-03-01T20:00:00Z

- hypothesis: CFLAGS/CXXFLAGS or rustflags target-feature=-crt-static can override esaxx-rs's CRT choice
  evidence: esaxx-rs build.rs calls cc::Build::new().static_crt(true) explicitly. The cc crate's static_crt() method directly sets /MT regardless of environment variables or Rust target features. CFLAGS and rustflags cannot override an explicit cc::Build method call.
  timestamp: 2026-03-01T20:02:00Z

## Evidence

- timestamp: 2026-03-01T19:02:00Z
  checked: tauri-build-2.5.5/src/static_vcruntime.rs
  found: When STATIC_VCRUNTIME=true env var is set, tauri-build injects: /NODEFAULTLIB for all dynamic CRT variants, /DEFAULTLIB:libcmt.lib (static C CRT), /DEFAULTLIB:libvcruntime.lib, /DEFAULTLIB:ucrt.lib. Also creates empty msvcrt.lib stub to override the hard-coded msvcrt.lib reference.
  implication: Release builds via npx tauri build set STATIC_VCRUNTIME=true (confirmed in Tauri CLI changelog). This forces static CRT for the final link.

- timestamp: 2026-03-01T19:04:00Z
  checked: target/release/build/voice-to-text-*/output
  found: Confirms /NODEFAULTLIB:msvcrt.lib + /DEFAULTLIB:libcmt.lib injected in release build.
  implication: The release build that succeeded (without parakeet, Mar 1 11:32) used static CRT. It worked because without onnxruntime.lib, nothing pulled in msvcprt.lib to conflict.

- timestamp: 2026-03-01T19:06:00Z
  checked: ort-sys-2.0.0-rc.10/build.rs + build output
  found: ort-sys downloads onnxruntime.lib (303MB static lib) from ort.pyke.io cache. It emits cargo:rustc-link-lib=static=onnxruntime. The lib path is AppData/Local/ort.pyke.io/dfbin/x86_64-pc-windows-msvc/.../onnxruntime/lib/onnxruntime.lib.
  implication: onnxruntime.lib is a LARGE static library (not an import lib for a DLL). When linked, it pulls in all its dependencies including msvcprt.lib (dynamic C++ STL).

- timestamp: 2026-03-01T19:07:00Z
  checked: dumpbin /directives on all whisper static libs + onnxruntime.lib
  found: All files have /FAILIFMISMATCH:RuntimeLibrary=MD_DynamicRelease and /DEFAULTLIB:msvcprt. None use static CRT. The conflict is between: (A) libcpmt.lib pulled in as C++ STL companion to tauri's forced libcmt.lib, and (B) msvcprt.lib pulled in by onnxruntime.lib's own /DEFAULTLIB directive.
  implication: The conflict is libcpmt.lib (static C++ STL) vs msvcprt.lib (dynamic C++ STL). Both define the same C++ standard library symbols.

- timestamp: 2026-03-01T19:09:00Z
  checked: tauri-build source, build output differences between debug and release
  found: Debug builds (npx tauri dev) do NOT have STATIC_VCRUNTIME set -> no libcmt.lib forced -> no CRT conflict. Release builds (npx tauri build) DO set it via Tauri CLI. The issue manifests on cargo build / npx tauri build with whisper+parakeet.
  implication: The fix should target release build linking. The symptom description says "npx tauri dev" but the actual failure occurs on release builds or when STATIC_VCRUNTIME is set.

- timestamp: 2026-03-01T19:11:00Z
  checked: ort 2.0.0-rc.10 Cargo.toml features list
  found: ort supports load-dynamic feature (uses libloading to load onnxruntime.dll at runtime). But the ort.pyke.io distribution only provides onnxruntime.lib (static) + DirectML.dll. No onnxruntime.dll is available in the cached package.
  implication: load-dynamic is not immediately usable without obtaining onnxruntime.dll separately.

- timestamp: 2026-03-01T20:01:00Z
  checked: esaxx-rs-0.1.10/build.rs in cargo registry
  found: Lines 7 and 20: cc::Build::new().static_crt(true). This is hardcoded in the published crate. The dependency chain is: parakeet-rs -> tokenizers -> esaxx-rs (with cpp feature enabled via tokenizers' esaxx_fast default feature).
  implication: LNK2038 is caused by esaxx.cpp being compiled /MT while everything else is /MD. /NODEFAULTLIB cannot suppress /FAILIFMISMATCH directives.

- timestamp: 2026-03-01T20:03:00Z
  checked: tokenizers-0.20.4/Cargo.toml
  found: esaxx-rs dep uses default-features=false, features=[]. But tokenizers' own default features include esaxx_fast = ["esaxx-rs/cpp"]. parakeet-rs uses tokenizers with no default-features override, so esaxx_fast (and thus esaxx-rs/cpp) is enabled.
  implication: Cannot disable the cpp feature from our Cargo.toml — it's a transitive default. The only fix is to patch the esaxx-rs source itself.

- timestamp: 2026-03-01T20:05:00Z
  checked: [patch.crates-io] approach: copied esaxx-rs-0.1.10 to patches/esaxx-rs/, changed .static_crt(true) -> .static_crt(false), added [patch.crates-io] esaxx-rs = { path = "patches/esaxx-rs" } to src-tauri/Cargo.toml
  found: cargo build --release --features "whisper,parakeet" with STATIC_VCRUNTIME=true completed in 2m 15s with no linker errors. dumpbin on the patched esaxx.lib shows RuntimeLibrary=MD_DynamicRelease, /DEFAULTLIB:msvcprt (not MT_StaticRelease).
  implication: Root cause resolved. The esaxx.lib now matches onnxruntime.lib and whisper libs — all /MD.

## Resolution

root_cause: esaxx-rs 0.1.10 (a transitive dependency: parakeet-rs -> tokenizers -> esaxx-rs) hardcodes .static_crt(true) in its build.rs, which compiles esaxx.cpp with /MT (MT_StaticRelease). All other C++ static libs in the link unit (onnxruntime.lib from ort-sys, whisper libs) use /MD (MD_DynamicRelease). MSVC LNK2038 is triggered by the /FAILIFMISMATCH:RuntimeLibrary directive embedded in each .obj — this is a hard linker fatal that cannot be suppressed with /NODEFAULTLIB. Adding parakeet-rs activated this previously-dormant dependency.

fix: Applied [patch.crates-io] in src-tauri/Cargo.toml to replace esaxx-rs-0.1.10 from crates.io with a local copy at patches/esaxx-rs/. The only change in the patch is .static_crt(true) -> .static_crt(false) in build.rs (both the non-macOS and macOS branches). This causes esaxx.cpp to be compiled with /MD, matching all other C++ libs. Reverted the previous incorrect build.rs workaround (NODEFAULTLIB:libcpmt.lib).

verification: STATIC_VCRUNTIME=true cargo build --release --features "whisper,parakeet" completed successfully (2m 15s, no linker errors). dumpbin on the patched esaxx.lib confirms RuntimeLibrary=MD_DynamicRelease. Both debug and release builds pass.

files_changed:
  - src-tauri/Cargo.toml (added [patch.crates-io] esaxx-rs section)
  - src-tauri/patches/esaxx-rs/build.rs (copied from registry, .static_crt(true) -> .static_crt(false))
  - src-tauri/build.rs (reverted to minimal tauri_build::build() only)
