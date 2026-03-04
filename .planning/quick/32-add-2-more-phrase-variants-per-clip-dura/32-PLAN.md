---
phase: quick-32
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - test-fixtures/generate-benchmark-wavs.ps1
  - src-tauri/src/bin/benchmark.rs
autonomous: true
requirements: [QUICK-32]
must_haves:
  truths:
    - "PowerShell script generates 9 WAV files: 3 durations x 3 variants"
    - "Benchmark binary loads and iterates all 9 clips"
    - "WER computed correctly for all 9 clips against their reference transcriptions"
    - "Markdown report file written to working directory after benchmark run"
  artifacts:
    - path: "test-fixtures/generate-benchmark-wavs.ps1"
      provides: "TTS generation for 9 benchmark WAV files"
      contains: "benchmark-5s-b.wav"
    - path: "src-tauri/src/bin/benchmark.rs"
      provides: "Benchmark binary with 9 clips and .md report output"
      contains: "REF_5S_B"
  key_links:
    - from: "test-fixtures/generate-benchmark-wavs.ps1"
      to: "src-tauri/src/bin/benchmark.rs"
      via: "WAV filenames and reference transcription text must match exactly"
      pattern: "benchmark-.*-[bc]\\.wav"
---

<objective>
Add 2 more phrase variants per clip duration (5s, 30s, 60s) to the benchmark suite, expanding from 3 to 9 WAV files, and add markdown report output to the benchmark binary.

Purpose: More diverse test content improves WER measurement reliability and reduces bias from self-referential benchmark phrases. The .md report provides a persistent record of results.
Output: Updated PowerShell generator (9 WAVs), updated benchmark.rs (9 clips + .md writer)
</objective>

<context>
@test-fixtures/generate-benchmark-wavs.ps1
@src-tauri/src/bin/benchmark.rs
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add 6 new phrase variants to PowerShell WAV generator</name>
  <files>test-fixtures/generate-benchmark-wavs.ps1</files>
  <action>
Add 6 new TTS sections to generate-benchmark-wavs.ps1 after the existing 3. Use the same `$synth`, `$format`, and output pattern. Naming: `benchmark-{duration}-{variant}.wav` where variant is `b` or `c`.

New phrases (real-world content, NOT about speech recognition or benchmarking):

**5s-b** (~5 seconds of speech):
"A copper wire carries electrical current through the circuit board with minimal resistance."

**5s-c** (~5 seconds of speech):
"The satellite orbits Earth every ninety minutes, capturing high resolution photographs."

**30s-b** (~30 seconds of speech):
```
The process of steel manufacturing begins with iron ore extraction from open pit mines.
Workers transport the raw material to blast furnaces where temperatures exceed fifteen hundred degrees.
Carbon is introduced to create an alloy stronger than pure iron alone.
Rolling mills then shape the molten steel into beams, sheets, and coiled wire.
Quality control inspectors test samples for tensile strength and corrosion resistance.
Modern foundries produce over two billion tonnes of steel worldwide each year.
```

**30s-c** (~30 seconds of speech):
```
Mediterranean cooking relies heavily on olive oil, fresh herbs, and seasonal vegetables.
Tomatoes were introduced to European cuisine after Spanish explorers returned from the Americas.
A traditional risotto requires constant stirring to release starch from Arborio rice grains.
Fermentation transforms grape juice into wine through the action of natural yeasts on sugar.
Sourdough bread uses a live culture of bacteria and wild yeast instead of commercial packets.
The Maillard reaction between amino acids and sugars creates the brown crust on grilled meat.
```

**60s-b** (~60 seconds of speech):
```
The Panama Canal connects the Atlantic and Pacific oceans through a series of concrete locks.
Construction began in nineteen oh four and took ten years to complete at enormous human cost.
Ships entering from the Atlantic side are raised twenty six metres above sea level by three lock chambers.
Gatun Lake was created by damming the Chagres River and flooding an entire valley.
Each lock chamber uses gravity fed water from the lake rather than mechanical pumps.
A single transit moves approximately two hundred million litres of fresh water into the ocean.
The canal was expanded in twenty sixteen with larger locks to accommodate modern container ships.
These new Neopanamax locks use water saving basins that recycle sixty percent of each fill.
Over fourteen thousand vessels pass through the canal annually carrying five percent of world trade.
Drought conditions in recent years have forced authorities to limit daily transits and vessel draft.
Tolls range from a few hundred dollars for small sailboats to nearly a million for the largest tankers.
The canal remains one of the most significant engineering achievements of the twentieth century.
Ongoing maintenance requires continuous dredging of the navigational channel to prevent silting.
Tropical rainfall patterns directly influence water levels in Gatun and Alajuela lakes.
The Panama Canal Authority employs over nine thousand workers to operate and maintain the waterway.
```

**60s-c** (~60 seconds of speech):
```
The human immune system consists of two complementary defence mechanisms working in coordination.
Innate immunity provides immediate but non specific protection through physical barriers and white blood cells.
Neutrophils are the first responders arriving at infection sites within minutes of tissue damage.
The adaptive immune system develops targeted responses through B cells and T cells over several days.
B cells produce antibodies that bind to specific molecular patterns on the surface of pathogens.
Helper T cells coordinate the overall immune response by releasing chemical signalling molecules called cytokines.
Memory cells persist for decades allowing the body to mount rapid responses to previously encountered threats.
Vaccination works by introducing harmless fragments of a pathogen to train the adaptive immune system.
Autoimmune disorders occur when the immune system mistakenly attacks the body's own healthy tissue.
Allergic reactions represent an exaggerated immune response to normally harmless environmental substances.
Immunosuppressive drugs are prescribed after organ transplants to prevent rejection of donor tissue.
The thymus gland plays a critical role in T cell maturation during childhood and adolescence.
Researchers continue developing immunotherapy treatments that harness the immune system to fight cancer cells.
The gut microbiome influences immune function through constant interaction with intestinal immune tissue.
Regular moderate exercise has been shown to enhance immune surveillance and reduce inflammation markers.
```

For each new variant, follow the same pattern as the existing clips:
1. Set output path: `$fileXX = Join-Path $OutputDir "benchmark-{dur}-{variant}.wav"`
2. Set output: `$synth.SetOutputToWaveFile($fileXX, $format)`
3. For 5s clips: `$synth.Speak("...")` with the phrase directly
4. For 30s/60s clips: define a here-string variable, then `$synth.Speak($variableName)`
5. Reset output: `$synth.SetOutputToNull()`
6. Print size info with same format string

Place `$synth.Dispose()` at the very end (move it after all new clips). Update the final "Done" message.
  </action>
  <verify>
    <automated>powershell -Command "& { $content = Get-Content 'C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text/test-fixtures/generate-benchmark-wavs.ps1' -Raw; $matches = [regex]::Matches($content, 'benchmark-\d+s(-[bc])?\.wav'); Write-Host ('WAV references found: ' + $matches.Count); if ($matches.Count -ge 9) { Write-Host 'PASS' } else { Write-Host 'FAIL'; exit 1 } }"</automated>
  </verify>
  <done>generate-benchmark-wavs.ps1 generates 9 WAV files (3 original + 6 new variants) with varied real-world content</done>
</task>

<task type="auto">
  <name>Task 2: Add new clip references and markdown report output to benchmark binary</name>
  <files>src-tauri/src/bin/benchmark.rs</files>
  <action>
Multiple changes to benchmark.rs:

**A. Add 6 new reference transcription constants** (lines ~260-283 area, after existing REF_60S):

Add `REF_5S_B`, `REF_5S_C`, `REF_30S_B`, `REF_30S_C`, `REF_60S_B`, `REF_60S_C` as `const &str` values. The text must match the PowerShell phrases EXACTLY (same words, same punctuation). Use the backslash line-continuation style for multi-line constants, matching the existing `REF_30S` / `REF_60S` pattern.

**B. Update `reference_for_clip()`** (line ~348) to handle new labels:

```rust
fn reference_for_clip(clip_label: &str) -> &'static str {
    match clip_label {
        "5s" => REF_5S,
        "5s-b" => REF_5S_B,
        "5s-c" => REF_5S_C,
        "30s" => REF_30S,
        "30s-b" => REF_30S_B,
        "30s-c" => REF_30S_C,
        "60s" => REF_60S,
        "60s-b" => REF_60S_B,
        "60s-c" => REF_60S_C,
        _ => "",
    }
}
```

**C. Update the `clips` vec** (line ~493) to include all 9 files:

```rust
let clips: Vec<(&str, &str)> = vec![
    ("benchmark-5s.wav",    "5s"),
    ("benchmark-5s-b.wav",  "5s-b"),
    ("benchmark-5s-c.wav",  "5s-c"),
    ("benchmark-30s.wav",   "30s"),
    ("benchmark-30s-b.wav", "30s-b"),
    ("benchmark-30s-c.wav", "30s-c"),
    ("benchmark-60s.wav",   "60s"),
    ("benchmark-60s-b.wav", "60s-b"),
    ("benchmark-60s-c.wav", "60s-c"),
];
```

**D. Add `use std::io::Write;`** at the top (needed for `write!` to file).

**E. Add markdown report generation** at the end of `print_summary()` (before the closing brace), or as a separate `write_markdown_report()` function called from `main()` right after `print_summary(&results)`:

Create a function `fn write_markdown_report(results: &[BenchResult])` that:

1. Opens `benchmark-results.md` for writing (in the current working directory, NOT test-fixtures/)
2. Writes a header: `# VoiceType Benchmark Results` with a timestamp line (`Generated: {date}`)
3. Writes a `## Results` section with a markdown table matching the stdout format:

```
| Model | Clip | Avg (ms) | Min (ms) | Max (ms) | WER % |
|-------|------|----------|----------|----------|-------|
| ... | ... | ... | ... | ... | ...% |
```

4. Writes a `## Model Rankings` section with the same scoring table from stdout (recompute or pass scored data). Include the same columns: Model, Avg Lat., Avg WER, Accuracy, Speed, Score. Sort by overall score descending.

5. Writes a `## Reference Transcriptions` section listing each clip label and its reference text:
```
### 5s
> The quick brown fox jumps over the lazy dog. Pack my box with five dozen liquor jugs.

### 5s-b
> A copper wire carries electrical current through the circuit board with minimal resistance.
```
(Use blockquotes for the reference text.)

6. Writes a `## Transcription Samples` section showing each model's first-run output per clip:
```
### whisper-small-en
- **5s**: "The quick brown fox..."
- **30s**: "Speech recognition..."
```

7. After writing, prints `"Wrote benchmark-results.md"` to stdout.

Call `write_markdown_report(&results)` from `main()` right after the `print_summary(&results)` call (line 1050).

For the timestamp, use `chrono` if available, otherwise use a simple approach:
```rust
let now = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_secs())
    .unwrap_or(0);
```
Or just omit the timestamp and use a static note like `Generated by: cargo run --bin benchmark`.

For the model rankings recomputation in the .md writer, extract the scoring logic from `print_summary` into a shared helper, OR simply duplicate the ranking computation in `write_markdown_report` (the function is self-contained and duplication is acceptable for a benchmark tool).

**F. Update the doc comment** at the top of the file (line 7) to mention that 9 WAV fixtures are now expected.
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text/src-tauri && cargo check --bin benchmark --features whisper,parakeet 2>&1 | tail -5</automated>
  </verify>
  <done>benchmark.rs compiles with all 9 clip references, correct reference_for_clip matching, and markdown report writer. Running the benchmark produces both stdout output and benchmark-results.md.</done>
</task>

</tasks>

<verification>
1. `cargo check --bin benchmark --features whisper,parakeet` compiles without errors
2. PowerShell script references all 9 WAV filenames
3. All 9 clip labels in benchmark.rs have corresponding REF_* constants
4. `reference_for_clip` covers all 9 labels (no fallthrough to empty string for valid clips)
</verification>

<success_criteria>
- generate-benchmark-wavs.ps1 defines 9 WAV outputs with varied real-world content
- benchmark.rs has 9 REF_* constants matching the PowerShell phrases exactly
- clips vec lists all 9 WAV files with correct labels
- reference_for_clip handles all 9 labels
- write_markdown_report function writes benchmark-results.md with results table, rankings, reference transcriptions, and transcription samples
- cargo check passes for the benchmark binary
</success_criteria>

<output>
After completion, create `.planning/quick/32-add-2-more-phrase-variants-per-clip-dura/32-SUMMARY.md`
</output>
