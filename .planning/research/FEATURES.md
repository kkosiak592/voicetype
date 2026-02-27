# Feature Research

**Domain:** Local voice-to-text desktop dictation tool (Windows, offline, hotkey-activated)
**Researched:** 2026-02-27
**Confidence:** HIGH (grounded in BridgeVoice docs, Wispr Flow feature page, multiple reference OSS projects, competitor analysis)

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist. Missing these = product feels incomplete.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Global hotkey activation | Every dictation tool uses system-wide shortcuts; no hotkey = tool is unusable | LOW | tauri-plugin-global-shortcut; must intercept regardless of focused app |
| Hold-to-talk (push-to-talk) mode | Natural for short bursts; avoids accidental recording; BridgeVoice primary mode | LOW | Start recording on keydown, transcribe on keyup |
| Toggle mode (press to start/stop) | Required for longer dictations where holding is fatiguing; BridgeVoice supports it | LOW | VAD for automatic cutoff; Silero detects silence |
| Floating visual indicator during recording | Users must know when mic is live; overlay is universal pattern (BridgeVoice, Wispr Flow, VoiceTypr all have it) | MEDIUM | Pill-shaped, always-on-top, frameless, transparent; Tauri transparent window workaround required |
| Audio level visualizer | Users want confirmation the mic is capturing; 7-bar frequency visualizer is BridgeVoice's pattern | MEDIUM | Web Audio API or cpal FFT in Rust; frequency bars update at ~30fps |
| Automatic text injection at cursor | Core value proposition — text must appear where the user is typing, not in a separate box | MEDIUM | Clipboard paste (Ctrl+V) is the proven path; BridgeVoice, Handy, OpenWhispr all use it |
| System tray presence | Background tool must live somewhere; tray icon is Windows convention for persistent background apps | LOW | tauri-plugin-tray-icon; context menu with Quit, Settings, version |
| Configurable hotkey | Different users have conflicting key preferences; forced default is a blocker | LOW | Settings panel UI; must validate for conflicts with common app shortcuts |
| Multiple Whisper model sizes | Not all machines have 5GB VRAM; users on laptops need small/medium; power users want large | MEDIUM | Model download on first run; GPU auto-detection with CPU fallback |
| Model download on first run | Bundling 1.5GB+ model in installer is impractical; NSIS fails for >2GB | LOW | HTTP download from Hugging Face or hosted CDN; progress bar in onboarding UI |
| Settings panel | Every production dictation tool has a settings screen (BridgeVoice, Wispr Flow, Superwhisper) | MEDIUM | Hotkeys, model selection, microphone choice, profiles; tauri-plugin-store for persistence |
| Word correction dictionary | Whisper consistently mishears domain terms; user-editable find-and-replace is universal in this category | MEDIUM | JSON/TOML config file; applied post-transcription; BridgeVoice calls it "Dictionary" |
| Clipboard restoration after paste | Overwrting the clipboard without restoring it is a widely-reported UX bug in dictation tools | LOW | Save clipboard before paste, restore after ~100ms delay; race condition if done too fast |

---

### Differentiators (Competitive Advantage)

Features that set the product apart. Not required, but valued.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Vocabulary profiles (domain-specific bundles) | Structural engineers, developers, legal — each needs different bias; no competitor bundles whisper initial_prompt + correction dictionary as a switchable profile | MEDIUM | Profile = whisper initial_prompt + regex corrections + case rules; switchable from tray or hotkey; ships with "Structural Engineering" and "General" |
| Structural engineering profile out of the box | Whisper without guidance mishears I-beam, W-section, MPa, rebar, AISC, ACI; having a profile pre-configured is unique in the market | MEDIUM | initial_prompt: "structural engineering, I-beam, W-section, rebar, prestressed concrete, kips, PSI, MPa, AISC, ACI 318"; regex: "why section" -> "W-section", "eye beam" -> "I-beam", "mega pascals" -> "MPa" |
| Caps lock output mode | Engineering drawing annotations and Bluebeam PDF markups are conventionally all-caps; no current open-source tool has a dedicated "ALL CAPS" output mode switchable per profile | LOW | Post-process: `text.toUpperCase()`; toggle per profile or per session; obvious differentiator for engineering/CAD workflows |
| Whisper initial_prompt support per profile | whisper.cpp --initial-prompt biases model toward specific vocabulary; combining it with post-processing corrections is more effective than either alone; Wispr Flow achieves similar results via LLM cleanup but is cloud-only | MEDIUM | whisper-rs exposes initial_prompt parameter; set per profile; dramatically improves domain accuracy without fine-tuning |
| Hotword/regex post-processing corrections | Level 2 corrections beyond simple find-and-replace; catches phonetic mishearings like "why section" that exact-match cannot fix | MEDIUM | Ordered list of regex patterns applied in sequence after transcription; user-editable in settings |
| Local-only privacy guarantee | Wispr Flow is cloud; Superwhisper is cloud-optional; BridgeVoice is local-only but macOS-only; a Windows local-only tool with BridgeVoice-quality UX is a gap | LOW | No telemetry, no network calls, fully offline; surface prominently in settings and tray tooltip |
| Silero VAD silence detection | Enables toggle mode to auto-stop without user pressing hotkey again; tools without VAD require manual stop which is clunky for long dictation | MEDIUM | ort (ONNX Runtime for Rust) running Silero 1.8MB model; 30ms chunks, ~1ms inference; configurable silence threshold and pad duration |
| CUDA 11.x compatibility | Most tools built for CUDA 12; P2000 owners (Pascal arch, common in enterprise/engineering workstations) are excluded by the market | HIGH | whisper.cpp with -DGGML_CUDA=1; MSVC + CUDA Toolkit 11.7; automatic CPU fallback for non-NVIDIA; this is a hardware-specific differentiator for the target user |
| Transcription history (local) | BridgeVoice logs every transcription with timestamp, word count, duration; useful for reviewing what was dictated, correcting errors, or re-injecting text | MEDIUM | SQLite or tauri-plugin-store JSON log; UI view in settings panel; "re-inject" button; not a v1 requirement but natural v1.x addition |

---

### Anti-Features (Commonly Requested, Often Problematic)

Features that seem good but create problems.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Streaming / real-time partial transcription | Feels more responsive; users see text appear as they speak | whisper.cpp does not natively stream; chunked streaming means poor accuracy on partial utterances (model lacks sentence context); dramatically increases architecture complexity; BridgeVoice deliberately chose chunk-based | Chunk-based with fast inference (<500ms) feels near-instant; Silero VAD cuts silence so chunks are tight |
| LLM-based text cleanup (Wispr Flow style) | Removes filler words, fixes grammar, formats output intelligently | Requires cloud API (Groq/OpenAI) which breaks offline premise; adds 200-800ms latency; can hallucinate and change meaning; overkill for technical dictation where accuracy matters more than prose cleanup | Whisper initial_prompt + regex corrections handles structural engineering vocabulary without LLM risk |
| Preview/confirmation step before paste | Lets user verify transcript before injecting | Destroys the workflow speed advantage; users stop and evaluate after every utterance; BridgeVoice explicitly chose instant injection | Trust transcription quality; provide easy undo via Ctrl+Z in target app; show recent in history |
| Context-aware app detection (screenshot capture) | Wispr Flow captures screenshots of active window to bias output per app | Privacy violation risk; Wispr Flow's screenshot feature is its most controversial aspect; not appropriate for a privacy-first local tool | Manual profile switching covers the 80% case; user selects "structural engineering" profile before starting session |
| Multi-language support | Broad appeal feature | Whisper already supports 100+ languages automatically; building UI around language selection adds settings complexity for zero additional accuracy; Whisper auto-detects language | Rely on Whisper's built-in auto-detection; expose language override as advanced setting only |
| Cloud/API transcription fallback | Handles cases where local model is slow or unavailable | Undermines the core privacy value proposition; adds API key management complexity; creates two code paths to maintain; cloud is not faster than a warmed-up local GPU anyway | Ship with two local models (GPU: large-v3-turbo, CPU: small); CPU performance is acceptable for the target use case |
| Voice commands for editing ("delete that", "new line") | Dragon-style correction via voice | Requires a command recognition layer separate from transcription; Whisper does not natively separate commands from content; dramatically increases complexity | Keyboard-based editing in target app; users are at keyboard anyway (hotkey activation) |
| Automatic microphone selection | Detect best mic automatically | Ambiguous when multiple mics exist (headset + webcam + desktop); wrong selection causes silent failures | Show mic list in settings with "currently selected" prominently displayed; default to Windows default mic |
| Word-for-word transcript display in overlay | Show transcription text in the floating pill widget | Pill overlay covering text creates readability problems; text length is unpredictable; existing tools (BridgeVoice, Wispr Flow) use minimal overlays — status only | Show state only (idle/recording/processing) in pill; let target application show the result |

---

## Feature Dependencies

```
[Configurable global hotkey]
    └──requires──> [Settings panel]
                       └──requires──> [tauri-plugin-store persistence]

[Hold-to-talk mode]
    └──requires──> [Audio capture (cpal)]
                       └──requires──> [Microphone permission handling]

[Toggle mode]
    └──requires──> [Audio capture (cpal)]
    └──requires──> [Silero VAD]  <-- auto-stop without VAD is broken UX
                       └──requires──> [ONNX Runtime (ort)]

[Text injection at cursor]
    └──requires──> [Clipboard save/restore]
    └──requires──> [Transcription output]
                       └──requires──> [whisper-rs + whisper.cpp]

[Vocabulary profiles]
    └──requires──> [Word correction dictionary]
    └──requires──> [Whisper initial_prompt support (whisper-rs)]
    └──requires──> [Caps lock output mode]  <-- one of profile's settings
    └──requires──> [Settings panel]

[Caps lock output mode]
    └──requires──> [Post-processing pipeline]

[Domain-specific profiles (Structural Engineering)]
    └──requires──> [Vocabulary profiles system]
    └──enhances──> [Whisper initial_prompt support]

[Floating pill overlay]
    └──requires──> [Tauri transparent/frameless window]
    └──requires──> [Audio visualizer data from cpal]

[Model download on first run]
    └──requires──> [GPU detection at startup]
    └──enhances──> [Model selection in settings]

[Transcription history (v1.x)]
    └──requires──> [Text injection at cursor]  <-- history stores what was injected
    └──requires──> [Settings panel]  <-- history viewer lives there
```

### Dependency Notes

- **Toggle mode requires Silero VAD:** Without VAD, toggle mode has no reliable silence detection and requires the user to always manually press stop. This degrades the experience to the point where toggle mode provides no advantage over hold-to-talk.
- **Vocabulary profiles require the correction dictionary system:** Profiles are bundles — you can't have a "Structural Engineering" profile without first having a working post-processing pipeline with configurable rules.
- **Caps lock mode is a property of a profile, not a standalone feature:** It must be implemented as a profile setting, not a global toggle, so users can have one profile that outputs ALL CAPS (for drawing annotations) and another that does not.
- **Floating pill requires transparent window workaround in Tauri v2:** GitHub issues #8308 and #13270 document the known Windows transparent window bug. This must be addressed in the same phase as the overlay UI.
- **Silero VAD conflicts with streaming transcription:** VAD-based endpoint detection is chunk-based by design. Implementing streaming would require removing VAD and replacing with a different architecture. These cannot coexist.

---

## MVP Definition

### Launch With (v1)

Minimum viable product — what's needed to validate the concept as a useful daily driver.

- [ ] Global hotkey (configurable) — without this the tool cannot be activated
- [ ] Hold-to-talk mode — simpler mode, no VAD dependency, validates core pipeline
- [ ] Audio capture (cpal, 16kHz) — prerequisite for everything
- [ ] whisper.cpp transcription via whisper-rs (GPU + CPU fallback) — core capability
- [ ] Clipboard paste text injection with save/restore — makes transcription useful
- [ ] Floating pill overlay with recording state indicator — required UX signal
- [ ] Audio visualizer in pill (frequency bars) — confirms mic is capturing
- [ ] System tray with Quit and Settings — background app convention
- [ ] Settings panel — hotkey config, model selection, microphone selection
- [ ] Model download on first run with progress UI — installer can't bundle 1.5GB model
- [ ] Word correction dictionary (JSON config + UI editor) — domain accuracy without this is unacceptable for engineering use
- [ ] Vocabulary profiles (structural engineering + general) — the primary differentiator; validates the profile concept
- [ ] Caps lock output mode as profile property — engineering drawing use case
- [ ] Silero VAD + toggle mode — enables hands-free longer dictation sessions

### Add After Validation (v1.x)

Features to add once core loop is validated.

- [ ] Transcription history log with re-inject — adds recovery path; add when users report wanting to retrieve previous dictations
- [ ] Regex-based post-processing corrections (level 2) — add when simple dictionary misses phonetic variants ("why section")
- [ ] Hotword support via whisper.cpp --hotwords — experimental; add if initial_prompt alone is insufficient for accuracy
- [ ] Quick-add to dictionary from system tray — convenience after seeing repeated errors
- [ ] NSIS installer with auto-update — distribute to colleagues after v1 is stable

### Future Consideration (v2+)

Features to defer until the tool is proven.

- [ ] Additional domain profiles (legal, medical, software development) — defer until engineering profile is validated
- [ ] Moonshine CPU streaming model — viable once Moonshine ONNX Windows support matures; 70-270ms latency on CPU is compelling but untested for this use case
- [ ] Per-app profile auto-switching — defer; manual switching covers the need with far less complexity
- [ ] Tauri auto-updater (tauri-plugin-updater) — defer until distribution expands beyond personal use

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Global hotkey activation | HIGH | LOW | P1 |
| Hold-to-talk mode | HIGH | LOW | P1 |
| Audio capture (cpal) | HIGH | LOW | P1 |
| whisper-rs transcription (GPU/CPU) | HIGH | HIGH | P1 |
| Clipboard paste injection | HIGH | LOW | P1 |
| Floating pill overlay | HIGH | MEDIUM | P1 |
| Audio visualizer | MEDIUM | MEDIUM | P1 |
| System tray | MEDIUM | LOW | P1 |
| Settings panel | HIGH | MEDIUM | P1 |
| Model download on first run | HIGH | MEDIUM | P1 |
| Word correction dictionary | HIGH | MEDIUM | P1 |
| Vocabulary profiles | HIGH | MEDIUM | P1 |
| Caps lock output mode | HIGH (for engineering) | LOW | P1 |
| Silero VAD + toggle mode | HIGH | MEDIUM | P1 |
| Configurable hotkey | MEDIUM | LOW | P1 |
| Transcription history | MEDIUM | MEDIUM | P2 |
| Regex post-processing | MEDIUM | MEDIUM | P2 |
| Hotword support | LOW | LOW | P2 |
| Quick-add to dictionary from tray | LOW | LOW | P2 |
| NSIS installer | MEDIUM | LOW | P2 |
| Moonshine CPU streaming | MEDIUM | HIGH | P3 |
| Per-app profile auto-switching | LOW | HIGH | P3 |

**Priority key:**
- P1: Must have for launch
- P2: Should have, add when possible
- P3: Nice to have, future consideration

---

## Competitor Feature Analysis

| Feature | BridgeVoice | Wispr Flow | Superwhisper | VoiceTypr | This Project |
|---------|-------------|------------|--------------|-----------|--------------|
| Processing | Local (whisper.cpp) | Cloud (proprietary) | Cloud + local | Local (whisper) | Local (whisper.cpp) |
| Platform | macOS | Mac/Win/iOS | macOS/iOS | macOS/Win | Windows-first |
| Hold-to-talk | Yes | Yes | Yes | Yes | Yes |
| Toggle mode | Yes | Yes | Yes | Yes | Yes |
| Floating pill overlay | Yes (7-bar viz) | Minimal indicator | Menu bar | Menu bar | Yes (7-bar viz, BridgeVoice pattern) |
| System tray | Yes | Yes | Yes (menu bar) | Yes (menu bar) | Yes |
| Word correction dictionary | Yes (Settings > Dictionary) | Personal dictionary (learns) | Custom vocabulary | Not documented | Yes (JSON editor + regex) |
| Domain profiles | No | Styles (tone only) | Custom modes (LLM prompts) | No | Yes (whisper prompt + corrections + case rules) |
| Caps lock output | No | No | Via custom mode | No | Yes (profile property) |
| Whisper initial_prompt | Not exposed as UI | N/A (cloud) | Via custom mode prompt | Not exposed | Yes (per profile) |
| LLM text cleanup | No | Yes (Llama cloud) | Yes (OpenAI/Anthropic) | Optional (Groq/Gemini) | No (deliberate) |
| Filler word removal | No | Yes | Yes | Optional | No (deliberate) |
| Transcription history | Yes (local) | Cloud history | Yes | Not documented | v1.x |
| Privacy | Zero telemetry | Cloud processing | Cloud-optional | No cloud storage | Zero telemetry, fully offline |
| GPU: CUDA 11.x | N/A (macOS) | N/A (cloud) | N/A (macOS) | Unverified | Yes (P2000, Pascal arch) |
| CPU fallback | Yes (smaller model) | N/A (cloud) | N/A | Yes | Yes (whisper small) |
| Installer size | Small | Electron-based | Mac App Store | Small | ~2.5MB + model download |
| Open source | No | No | No | No | Yes (planned) |

---

## Sources

- [BridgeVoice Documentation](https://docs.bridgemind.ai/docs/bridgevoice) — PRIMARY: recording modes, widget states, dictionary, history, model sizes (HIGH confidence)
- [BridgeVoice Product Page](https://www.bridgemind.ai/products/bridgevoice) — feature list verification (HIGH confidence)
- [Wispr Flow Features Page](https://wisprflow.ai/features) — filler removal, punctuation, snippets, styles, developer mode (HIGH confidence)
- [Wispr Flow Technical Challenges](https://wisprflow.ai/post/technical-challenges) — streaming architecture, latency targets (MEDIUM confidence)
- [Superwhisper Introduction](https://superwhisper.com/docs/get-started/introduction) — modes, custom prompts, menu bar (MEDIUM confidence)
- [VoiceTypr GitHub](https://github.com/moinulmoin/voicetypr) — feature list from README: GPU, hotkey, visual feedback, AI enhancement (MEDIUM confidence)
- [OpenWhispr](https://openwhispr.com/) — custom dictionary with auto-learn, cascading paste, cross-platform (MEDIUM confidence)
- [Whispering (HN discussion)](https://news.ycombinator.com/item?id=44942731) — open-source local-first dictation, community feedback on features (MEDIUM confidence)
- [whisper.cpp GitHub](https://github.com/ggml-org/whisper.cpp) — initial_prompt and hotwords parameters (HIGH confidence)
- Artifacts: `artifacts/research/2026-02-27-voice-to-text-desktop-tool-technical.md` — existing deep technical research on the same project (HIGH confidence, primary author's research)

---
*Feature research for: Local voice-to-text desktop dictation tool (Windows)*
*Researched: 2026-02-27*
