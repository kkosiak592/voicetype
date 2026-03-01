$env:LIBCLANG_PATH = [System.Environment]::GetEnvironmentVariable('LIBCLANG_PATH', 'User')
$env:BINDGEN_EXTRA_CLANG_ARGS = [System.Environment]::GetEnvironmentVariable('BINDGEN_EXTRA_CLANG_ARGS', 'User')
$env:CMAKE_CUDA_ARCHITECTURES = '61'
$env:CUDA_PATH = 'C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9'

Write-Output "LIBCLANG_PATH: $env:LIBCLANG_PATH"
Write-Output "BINDGEN_EXTRA_CLANG_ARGS: $env:BINDGEN_EXTRA_CLANG_ARGS"
Write-Output "CMAKE_CUDA_ARCHITECTURES: $env:CMAKE_CUDA_ARCHITECTURES"

Set-Location 'C:\Users\kkosiak.TITANPC\Desktop\Code\voice-to-text'
& 'C:\Users\kkosiak.TITANPC\.cargo\bin\cargo.exe' build --manifest-path src-tauri/Cargo.toml --features whisper 2>&1 | Select-Object -Last 30
