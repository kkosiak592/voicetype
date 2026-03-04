---
phase: quick-36
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - test-fixtures/generate-benchmark-wavs.ps1
  - src-tauri/src/bin/benchmark.rs
autonomous: true
requirements: [Q36]
must_haves:
  truths:
    - "Running generate-benchmark-wavs.ps1 produces 12 WAV files including benchmark-90s.wav, benchmark-90s-b.wav, benchmark-90s-c.wav"
    - "Benchmark binary discovers and runs 90s/90s-b/90s-c clips alongside existing 5s/30s/60s clips"
    - "Pivot tables in both console and markdown output show a 90s column"
  artifacts:
    - path: "test-fixtures/generate-benchmark-wavs.ps1"
      provides: "TTS generation for 90s, 90s-b, 90s-c WAV clips"
      contains: "benchmark-90s"
    - path: "src-tauri/src/bin/benchmark.rs"
      provides: "REF_90S/REF_90S_B/REF_90S_C constants, clip entries, duration_groups with 90s"
      contains: "REF_90S"
  key_links:
    - from: "generate-benchmark-wavs.ps1"
      to: "benchmark.rs reference_for_clip()"
      via: "TTS phrase text must match REF_90S* constants exactly"
      pattern: "REF_90S"
---

<objective>
Add 90-second benchmark clips (90s, 90s-b, 90s-c) to the benchmark binary and WAV generation script.

Purpose: Extend benchmark coverage to 90-second clips for measuring sustained-load transcription performance beyond the current 60-second maximum.
Output: Updated PowerShell script generating 12 WAVs (was 9), updated benchmark.rs with 90s reference transcriptions, clip entries, and 4-column pivot tables.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@test-fixtures/generate-benchmark-wavs.ps1
@src-tauri/src/bin/benchmark.rs
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add 90s TTS phrases to PowerShell WAV generation script</name>
  <files>test-fixtures/generate-benchmark-wavs.ps1</files>
  <action>
Add three new 90-second clip sections to generate-benchmark-wavs.ps1, following the existing pattern (comment header, passage variable, SetOutputToWaveFile, Speak, SetOutputToNull, size printout). Place them after the 60s-c section and before `$synth.Dispose()`.

90s variant A topic: deep-sea oceanography (hydrothermal vents, pressure, marine biology). Write ~22 sentences of factual, TTS-friendly prose (no abbreviations, no special characters, numbers spelled out). This should produce approximately 90 seconds of Windows TTS speech at default rate.

90s variant B topic: history of aviation (Wright brothers through jet age). Same length ~22 sentences.

90s variant C topic: renewable energy systems (solar, wind, grid storage, efficiency). Same length ~22 sentences.

File naming: benchmark-90s.wav, benchmark-90s-b.wav, benchmark-90s-c.wav

Update the final Write-Host line from "9 WAV files" to "12 WAV files".

IMPORTANT: The TTS passage text in the PowerShell script must EXACTLY match the REF_90S* constants created in Task 2 (same words, same punctuation). Write both files with identical text.
  </action>
  <verify>
    <automated>powershell -Command "& { $content = Get-Content 'C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text/test-fixtures/generate-benchmark-wavs.ps1' -Raw; if ($content -match 'benchmark-90s\.wav' -and $content -match 'benchmark-90s-b\.wav' -and $content -match 'benchmark-90s-c\.wav' -and $content -match '12 WAV files') { Write-Host 'PASS: All 90s clips and count present' } else { Write-Host 'FAIL'; exit 1 } }"</automated>
  </verify>
  <done>PowerShell script has 3 new 90s clip sections with unique factual passages, file count updated to 12</done>
</task>

<task type="auto">
  <name>Task 2: Add 90s reference transcriptions, clip entries, and pivot table columns to benchmark.rs</name>
  <files>src-tauri/src/bin/benchmark.rs</files>
  <action>
Four changes to benchmark.rs:

1. **Add REF_90S, REF_90S_B, REF_90S_C constants** (after REF_60S_C around line 342). Use the same Rust multi-line string literal pattern (`const REF_90S: &str = "sentence one. \` with backslash continuations). Text must EXACTLY match the TTS passages from Task 1.

2. **Add match arms in reference_for_clip()** (around line 417, before the `_ => ""` catch-all):
   ```
   "90s" => REF_90S,
   "90s-b" => REF_90S_B,
   "90s-c" => REF_90S_C,
   ```

3. **Add clip entries to the `clips` vector** (around line 629, after the 60s-c entry):
   ```
   ("benchmark-90s.wav",   "90s"),
   ("benchmark-90s-b.wav", "90s-b"),
   ("benchmark-90s-c.wav", "90s-c"),
   ```

4. **Update pivot tables to include "90s" column** — there are 6 locations to update:

   a. **print_summary duration_groups** (line ~1892): change `["5s", "30s", "60s"]` to `["5s", "30s", "60s", "90s"]`

   b. **print_summary latency header** (line ~1916): change format to 5 columns:
      `println!("{:<30} | {:>10} | {:>10} | {:>10} | {:>10}", "Model", "5s", "30s", "60s", "90s");`

   c. **print_summary latency row** (line ~1925): change to:
      `println!("{:<30} | {:>10} | {:>10} | {:>10} | {:>10}", model, cols[0], cols[1], cols[2], cols[3]);`

   d. **print_summary WER header** (line ~1934): same 5-column format as (b)

   e. **print_summary WER row** (line ~1943): same 5-column format as (c)

   f. **print_summary separator lines**: change `.repeat(68)` to `.repeat(82)` (4 locations — the dashes and equals for both latency and WER tables). The extra 14 chars = " | " (3) + column width (10) + padding (1).

   g. **write_markdown_report duration_groups** (line ~2065): change `["5s", "30s", "60s"]` to `["5s", "30s", "60s", "90s"]`

   h. **write_markdown_report latency header** (line ~2076): change to `"| Model | 5s | 30s | 60s | 90s |"`

   i. **write_markdown_report latency separator** (line ~2077): change to `"|-------|----|-----|-----|-----|"`

   j. **write_markdown_report latency row** (line ~2085): change to:
      `let _ = writeln!(file, "| {} | {} | {} | {} | {} |", name, cols[0], cols[1], cols[2], cols[3]);`

   k. **write_markdown_report WER header** (line ~2089): same as (h)

   l. **write_markdown_report WER separator** (line ~2090): same as (i)

   m. **write_markdown_report WER row** (line ~2098): same as (j)

   n. **clip_labels array for Reference Transcriptions section** (line ~2103): add 90s labels:
      `let clip_labels = ["5s", "5s-b", "5s-c", "30s", "30s-b", "30s-c", "60s", "60s-b", "60s-c", "90s", "90s-b", "90s-c"];`

Note on prefix filtering: The existing `avg_metric` / `avg_metric_md` closures use `starts_with(prefix) && !clip[prefix.len()..].starts_with("s")` which correctly handles "90s" prefix (remainder for clips "90s"/"90s-b"/"90s-c" is ""  / "-b" / "-c", none starting with "s"). No changes needed to the filtering logic. However, verify that "9" won't accidentally be a prefix — it won't because duration_groups uses "90s" not "9".
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text/src-tauri && cargo check --bin benchmark --features bench_extra 2>&1 | tail -5</automated>
  </verify>
  <done>benchmark.rs compiles cleanly with REF_90S/REF_90S_B/REF_90S_C constants, reference_for_clip returns them for 90s/90s-b/90s-c, clips vector has 12 entries, both print_summary and write_markdown_report pivot tables show 4 duration columns (5s/30s/60s/90s), clip_labels includes all 12 labels</done>
</task>

</tasks>

<verification>
1. `cargo check --bin benchmark --features bench_extra` compiles without errors
2. grep for REF_90S, REF_90S_B, REF_90S_C in benchmark.rs confirms constants exist
3. grep for `"90s"` in benchmark.rs shows entries in clips vector, reference_for_clip match, duration_groups, and clip_labels
4. grep for `benchmark-90s` in generate-benchmark-wavs.ps1 confirms all 3 variants
5. PowerShell script final line says "12 WAV files"
</verification>

<success_criteria>
- benchmark.rs compiles with bench_extra feature
- 3 new REF_90S* constants with ~22 sentences each
- reference_for_clip() handles "90s", "90s-b", "90s-c"
- clips vector has 12 entries (was 9)
- Both pivot tables (console and markdown) have 4 duration columns
- clip_labels array has 12 entries for Reference Transcriptions section
- PowerShell script generates benchmark-90s.wav, benchmark-90s-b.wav, benchmark-90s-c.wav
- TTS passages in .ps1 exactly match REF_90S* constants in .rs
</success_criteria>

<output>
After completion, create `.planning/quick/36-add-90s-benchmark-clips-to-benchmark-bin/36-SUMMARY.md`
</output>
