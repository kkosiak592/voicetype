fn main() {
    tauri_build::build();

    // On Windows with MSVC, delay-load nvcuda.dll so the binary can start on
    // non-NVIDIA machines. Without this, Windows resolves nvcuda.dll at process
    // startup (it's an implicit import from whisper-rs's CUDA linkage), which
    // immediately fails on machines without NVIDIA GPU drivers installed.
    //
    // With delay-load, nvcuda.dll is only resolved the first time a CUDA driver
    // function is actually called. Since we check for NVIDIA GPU at runtime
    // (via NVML) before ever entering whisper's CUDA path, nvcuda.dll is never
    // touched on non-NVIDIA machines, and the app starts correctly.
    //
    // delayimp.lib provides the __delayLoadHelper2 runtime helper required by the
    // /DELAYLOAD mechanism.
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_os == "windows" && target_env == "msvc" {
        println!("cargo:rustc-link-arg=/DELAYLOAD:nvcuda.dll");
        println!("cargo:rustc-link-arg=delayimp.lib");
    }
}
