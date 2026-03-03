Add-Type -AssemblyName System.Speech

$OutputDir = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $OutputDir) { $OutputDir = "." }

# 16kHz, 16-bit, mono PCM — the format whisper-rs and parakeet-rs expect
$format = [System.Speech.AudioFormatInfo]::new(
    [System.Speech.Synthesis.EncodingFormat]::Pcm,
    16000,  # samples per second
    16,     # bits per sample
    1       # channels (mono)
)

$synth = New-Object System.Speech.Synthesis.SpeechSynthesizer

# -----------------------------------------------------------------------
# 5-second clip — short phrase for quick-latency testing
# -----------------------------------------------------------------------
$file5s = Join-Path $OutputDir "benchmark-5s.wav"
$synth.SetOutputToWaveFile($file5s, $format)
$synth.Speak("The quick brown fox jumps over the lazy dog. Pack my box with five dozen liquor jugs.")
$synth.SetOutputToNull()

$size5s = (Get-Item $file5s).Length
Write-Host ("benchmark-5s.wav  -> {0} bytes  ({1:F1}s expected at 16kHz/16bit/mono)" -f $size5s, ($size5s / (16000 * 2)))

# -----------------------------------------------------------------------
# 60-second clip — longer passage for sustained-load testing
# -----------------------------------------------------------------------
$passage = @"
Voice dictation software converts spoken words into written text in real time.
Modern systems use deep learning models trained on thousands of hours of audio data.
Accuracy depends on microphone quality, background noise, and speaking pace.
The whisper model was released by OpenAI and is widely used for offline transcription.
Parakeet is an NVIDIA model optimised for real-time inference on CUDA hardware.
To benchmark these models fairly, we measure wall-clock latency across multiple runs.
We test both a short five-second clip and a longer sixty-second passage.
Results include the average, minimum, and maximum inference time in milliseconds.
Lower latency means faster transcription and a better user experience.
Sub five hundred millisecond latency is generally imperceptible to the user.
English language models tend to be smaller and faster than multilingual alternatives.
Quantised models use reduced precision weights to run faster with minimal accuracy loss.
The Q5 underscore 1 format stores each weight in approximately five bits.
GPU acceleration can reduce inference time by ten times compared to CPU-only execution.
This benchmark helps select the best model for a given hardware configuration.
"@

$file60s = Join-Path $OutputDir "benchmark-60s.wav"
$synth.SetOutputToWaveFile($file60s, $format)
$synth.Speak($passage)
$synth.SetOutputToNull()

$size60s = (Get-Item $file60s).Length
Write-Host ("benchmark-60s.wav -> {0} bytes  ({1:F1}s expected at 16kHz/16bit/mono)" -f $size60s, ($size60s / (16000 * 2)))

$synth.Dispose()
Write-Host "Done. Files written to: $OutputDir"
