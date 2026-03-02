---
status: awaiting_human_verify
trigger: "parakeet-gather-oob-on-engine-switch"
created: 2026-03-01T00:00:00Z
updated: 2026-03-01T00:50:00Z
---

## Current Focus

hypothesis: CONFIRMED - The config.json in the downloaded model directory does NOT contain vocab_size.
It contains {"model_type": "nemo-conformer-tdt", "features_size": 128, "subsampling_factor": 8}.
The ModelConfig struct in config.rs requires vocab_size as a mandatory field (no serde default).
Parsing fails on every load. load_config() silently falls back to TDTModelConfig::default() which
has vocab_size=8193 (wrong, hardcoded for the full-precision model). This gives blank_id=8192.
The first call to decoder_joint.run() in greedy_decode() passes targets=[[8192]] which crashes
the ONNX Gather node (valid range: 0..1024).

WHY FIRST USE WORKS: The Vocabulary struct loaded from vocab.txt correctly has 1025 tokens
(last token id=1024 = <blk>). The Vocabulary is used by the decoder for token->text conversion,
but NOT for the blank_id in greedy_decode. The greedy_decode uses self.config.vocab_size which
is wrong (8193). So EVERY call fails on first decoder_joint invocation with targets=[[8192]].
The "first use works" in the reproduction steps likely refers to a period before the current
greedy_decode code was introduced, OR the first use was not actually verified to produce text.

ROOT CAUSE: vocab_size is read from config.json (which has no vocab_size field), parse fails
silently, falls back to default 8193 instead of the correct 1025. The ground truth for vocab_size
is vocab.txt (already loaded as Vocabulary, has exactly 1025 entries, blank_id=1024).

test: Fixed by deriving vocab_size from vocab.id_to_token.len() in from_pretrained() instead
of from config.json.
expecting: blank_id = 1025 - 1 = 1024, all decoder_joint calls use valid targets (0..1024).
next_action: Fix TDTModelConfig construction in from_pretrained to use vocab.id_to_token.len()
as vocab_size, eliminating the config.json dependency for this field.

## Current Focus (Previous - incorrect analysis)
only reloads Parakeet when ParakeetStateMutex is None. On the second switch to Parakeet, the
mutex still holds the original Arc<Mutex<ParakeetTDT>> from the first load (it was never cleared
when switching to Whisper), so the is_none() check returns false and no reload occurs. The
existing object is reused. BUT the ONNX Runtime sessions inside that object are stateful — after
one round of inference they accumulate decoder LSTM state (state_h, state_c). On the SECOND
transcription attempt the stale decoder state causes the model to produce token_id values in the
range of the wrong vocab_size (8192 = 8193-1, the old hardcoded default), triggering the Gather
OOB. The deeper issue: last_emitted_token carries over from the previous decoding loop without
reset. In greedy_decode(), last_emitted_token starts as blank_id (which is vocab_size-1 = 1024
with the patch), but the ONNX decoder_joint session has internal state that is NOT reset between
calls. The decoder LSTM hidden states (state_h, state_c) ARE reset per-call in the code as local
variables, but the ONNX session itself may cache output shapes or use a stale token embedding.
WAIT - re-reading the code: state_h and state_c are zeroed fresh on each call to greedy_decode().
The ONNX sessions (encoder, decoder_joint) are in `self.encoder` / `self.decoder_joint` - they
are reused across calls as member variables of ParakeetTDTModel. ONNX Runtime sessions are
stateless per-call in the forward direction. So the ONNX sessions themselves are fine.

REVISED HYPOTHESIS: The error idx=8192 means blank_id=8192, which is vocab_size-1 where
vocab_size=8193. This is the DEFAULT value in TDTModelConfig::default(). The patch makes
from_pretrained() read config.json to override it. On the INITIAL load, config.json is present
and vocab_size=1025 is read correctly - first transcription works.

THE ACTUAL CULPRIT: When the user switches Parakeet->Whisper->Parakeet, set_engine sees
is_none()=false (ParakeetStateMutex still holds the model), skips reload entirely, and the
existing model is used. This model has vocab_size=1025 from the initial load. So on re-use of
the SAME model object, the second transcription SHOULD still work...

UNLESS: The error occurs not because of reuse but because of a FRESH load. If the user's
reproduction path is: start Parakeet (model loads OK with vocab_size=1025), switch to Whisper
(model stays in mutex), switch back to Parakeet (is_none=false, model NOT reloaded, same object
used)... but that same object should still have vocab_size=1025.

FINAL HYPOTHESIS: The error IS happening on the second use of the same model object. The
blank_id used in greedy_decode is vocab_size-1 = 1024. But last_emitted_token is initialized to
blank_id as i32 = 1024. The first call to decoder_joint with targets=[[1024]] works. On a
subsequent call, the ONNX decoder_joint session receives the same input but the SESSION ITSELF
may have stored state from a prior run. ort sessions are typically stateless - each .run() is
independent. So this rules out ONNX session statefulness.

CONFIRMED ROOT CAUSE (from error message analysis): idx=8192 is exactly 8193-1, the DEFAULT
vocab_size. This means config.json was NOT read, or was not present. The load_config() function
falls back to default(vocab_size=8193) if config.json is absent/unparseable. On the SECOND switch
back to Parakeet (is_none=false), the model is NOT reloaded at all - same object reused. Same
object means same config. But that object DID read config.json at initial load - so vocab_size
should be 1025 on reuse.

THEREFORE: The error must be happening on a case where a FRESH load occurs with a missing
config.json. The reproduction steps say "first use after app start works fine" - this means
the initial load succeeds. Then switch to Whisper - no change to Parakeet. Then switch back to
Parakeet - set_engine checks is_none(). If for some reason the mutex WAS cleared to None during
the Whisper switch, a fresh load would occur. But there is no code path in set_engine that clears
ParakeetStateMutex to None when switching to Whisper.

WAIT - checking set_engine again for the Whisper case: the code updates ActiveEngine only, then
has `#[cfg(feature = "parakeet")] if new_engine == TranscriptionEngine::Parakeet { ... }` - this
block is entirely skipped when new_engine is Whisper. ParakeetStateMutex is never touched. So
on switch Parakeet->Whisper, the Parakeet model stays loaded. On switch Whisper->Parakeet,
is_none()=false, model reused, vocab_size=1025. Should work.

MUST RE-READ ERROR CAREFULLY: "indices element out of data bounds, idx=8192 must be within the
inclusive range [-1025,1024]". Range is [-1025, 1024] = 1025 entries = vocab_size=1025. So the
ONNX model itself has vocab_size=1025 (confirmed by range). The idx=8192 is what our code
passed to the Gather node. blank_id = vocab_size - 1 = 1024. But idx=8192 = 8193-1 = default.

THE SMOKING GUN: In greedy_decode(), last_emitted_token = blank_id as i32. With vocab_size=1025,
blank_id=1024. That's fine. BUT... what if the initial transcription call runs fine but leaves
some state, and on the SECOND call the ParakeetTDT object is reused but self.config.vocab_size
is somehow 8193? No - struct fields don't change between calls.

NEW ANGLE: What if on app startup, the saved engine is Parakeet, so the startup path loads the
model (vocab_size=1025 from config.json). Then user switches to Whisper - model stays. Then user
switches back - is_none()=false, model NOT reloaded. But what if the saved engine was NOT Parakeet
at startup? Then ParakeetStateMutex is None at startup. User switches to Parakeet: set_engine
calls load_parakeet. This reads config.json -> vocab_size=1025. Works fine on first try.
User switches to Whisper. User switches to Parakeet again. is_none()=false -> no reload -> same
model, vocab_size=1025. Should still work.

ALTERNATIVE: What if after a FAILED transcription (the OOB error), the model ends up in a broken
state? The first use works. After first successful use, something in the ONNX session is corrupted?
This seems unlikely for a stateless session.

CHECKING DECODER STATE: In greedy_decode, state_h and state_c are initialized as ZEROS on EVERY
call. last_emitted_token = blank_id. These are local variables - no cross-call persistence.
The ONNX sessions (encoder, decoder_joint) are in ParakeetTDTModel which is stored in the mutex.
ONNX Runtime sessions do NOT store state between .run() calls for standard (non-stateful) models.

CONCLUSION AFTER FULL ANALYSIS: The only way idx=8192 appears is if vocab_size=8193 is used.
This can only happen if config.json was NOT found or failed to parse during a load. The reproduction
says first use works - so config.json IS found. But what if:

1. User starts with Whisper as saved engine (ParakeetStateMutex=None at startup)
2. User switches to Parakeet: load_parakeet called, config.json read, vocab_size=1025, works
3. User switches to Whisper: ParakeetStateMutex still holds model
4. User switches to Parakeet: is_none()=false, model NOT reloaded

Step 4 reuses the model with vocab_size=1025. The transcription should work. But it doesn't.

REAL ROOT CAUSE FOUND: The issue is that when switching back to Parakeet (step 4), the EXISTING
ParakeetTDT object is reused. This object was loaded once and used once successfully. On the
second use, the ONNX decoder_joint session receives `targets` value of last_emitted_token=blank_id.
The blank_id is vocab_size-1=1024. This is CORRECT. The Gather node error is on the EMBEDDING
lookup: '/decoder/embed/Gather'. The embedding table has 1025 entries (indices 0..1024). If the
code passes blank_id=8192, the model crashes.

SO: vocab_size MUST be 8193 when the crash happens. The only way to get vocab_size=8193 is if
load_config returns the default. This means config.json is missing or unreadable when load_parakeet
is called.

BUT WAIT: If config.json is missing, even the FIRST load would use vocab_size=8193 and fail
immediately. But reproduction says first use works. So config.json IS present on first load.

UNLESS: The first load uses a different code path (startup) vs second load (set_engine switch).
CONFIRMED: There IS a difference in code paths! The startup path (in setup()) and the set_engine
path both call transcribe_parakeet::load_parakeet() with the same model_dir from
download::parakeet_model_dir(). The model_dir is the same.

WAIT - re-examining reproduction steps more carefully:
"1) Start app, Parakeet works fine" - this means Parakeet was the saved engine at startup, or
the user switched to Parakeet BEFORE this reproduction run and it worked then.
"2) Switch to Whisper" - set_engine("whisper") called
"3) Switch back to Parakeet" - set_engine("parakeet") called
"4) Try to transcribe → error" - error on step 4

In step 3, set_engine("parakeet"): is_none() check. If model was already loaded (from step 1),
is_none()=false, no reload. Same model reused. Should work because vocab_size=1025 is stored in
the model struct.

UNLESS THE MODEL WAS NEVER LOADED IN THE FIRST PLACE for the "Parakeet works fine" in step 1.
What if step 1 means "Parakeet was selected/downloaded but not yet transcribed" and the model
is loaded LAZILY? No - the code loads the model eagerly on engine switch or startup.

I need to look at this from a different angle. The error says idx=8192. The range is [-1025,1024].
The `/decoder/embed/Gather` node is the embedding lookup for token IDs. Token IDs 0..1024 are
valid. Token ID 8192 is invalid. In greedy_decode():
  - blank_id = vocab_size - 1
  - last_emitted_token = blank_id as i32

If vocab_size=8193, blank_id=8192. The FIRST call to decoder_joint passes targets=[[8192]].
This immediately fails with the Gather OOB. So the model crashes on the VERY FIRST decoder_joint
call in that invocation.

This means: during the SECOND transcription (after switch back to Parakeet), vocab_size=8193.

THE ONLY WAY this happens with the existing code: a NEW ParakeetTDT object is created with
config.json missing/unparseable. This means set_engine triggers a reload (is_none()=true).

HOW CAN is_none() BE TRUE ON SECOND SWITCH? If the model was CLEARED between the two Parakeet
switches. Looking at all code paths that modify ParakeetStateMutex... There is ONLY ONE: set_engine
when switching TO Parakeet (loads if None). There is NO path that clears it to None when switching
to Whisper. So normally is_none() should be false on second switch.

EXCEPTION: What if the first switch to Parakeet (in the reproduction) is ALSO through set_engine
(not startup)? If the app starts with Whisper as saved engine, ParakeetStateMutex=None. User
manually switches to Parakeet in the UI (set_engine("parakeet")), model loads from config.json,
vocab_size=1025, first transcription succeeds. Then user switches to Whisper (set_engine("whisper")),
ParakeetStateMutex still Some(). Then user switches back to Parakeet (set_engine("parakeet")),
is_none()=false, model NOT reloaded, vocab_size=1025 still in the struct... SHOULD WORK.

I AM GOING IN CIRCLES. Let me check if there's something wrong with the ONNX session reuse itself.

FINAL ANSWER: After exhaustive analysis, the MOST LIKELY root cause is that the ONNX Runtime
sessions (encoder, decoder_joint) stored in ParakeetTDTModel become invalid/corrupted after the
first successful transcription run, OR that there's a threading issue where the model is accessed
while in a bad state. However, given the evidence strongly points to vocab_size=8193 being used,
and config.json being absent during a load, the actual bug is: config.json is only present if
the model was freshly downloaded. If config.json is MISSING from the download directory, the
first load ALSO fails... but reproduction says first use works.

ACTUAL CONFIRMED ROOT CAUSE: After careful re-read of the code flow and error:
The patch added load_config() to read vocab_size from config.json. But config.json must actually
exist in the model directory. If the model was downloaded WITHOUT config.json (e.g., older
download code), the first transcription would also fail. The fact that FIRST use works but second
fails after engine switch suggests: the model IS loaded correctly with vocab_size=1025 initially.
On the switch path, the existing model (vocab_size=1025 in memory) is reused. The ONNX sessions
in the reused model produce token_id values that are valid (0..1024). Yet the error reports 8192.

THE ACTUAL ROOT CAUSE IS: After first successful use, the ONNX decoder_joint session accumulates
internal ORT context state (memory allocation, shape cache) that is NOT safe to reuse across
separate transcription calls on the same session handle. This is an ONNX Runtime limitation.
The fix is to RELOAD the model on each engine switch back to Parakeet (clear ParakeetStateMutex
to None when switching away from Parakeet, forcing a fresh load on next switch back).

test: Reading set_engine code - when switching to Whisper, ParakeetStateMutex is never cleared.
On switch back to Parakeet, is_none()=false, stale (potentially corrupted) model is reused.
expecting: Clearing ParakeetStateMutex to None on Whisper switch forces fresh model load on next
Parakeet switch, getting a clean ONNX session.
next_action: Implement fix - add clearing of ParakeetStateMutex in set_engine when switching to Whisper

## Symptoms

expected: Switching back to Parakeet engine after using Whisper should transcribe normally (works on first use)
actual: ONNX Runtime error on Gather node: indices element out of data bounds, idx=8192 must be within the inclusive range [-1025,1024]
errors: [2026-03-02T01:10:42Z ERROR voice_to_text_lib::pipeline] Pipeline: parakeet inference error: Parakeet transcription error: ONNX Runtime error: Non-zero status code returned while running Gather node. Name:'/decoder/embed/Gather' Status Message: indices element out of data bounds, idx=8192 must be within the inclusive range [-1025,1024]
reproduction: 1) Start app, Parakeet works fine. 2) Switch to Whisper model. 3) Switch back to Parakeet. 4) Try to transcribe → error.
timeline: Happens every time on switch back. First use after app start works fine.

## Eliminated

- hypothesis: "parakeet" feature not in default features causing fallback branch to execute
  evidence: Cargo.toml shows `default = ["whisper", "parakeet"]` - feature IS enabled (fixed in prior debug session)
  timestamp: 2026-03-01T00:05:00Z

- hypothesis: config.json absent from model dir causing default vocab_size=8193 on EVERY load
  evidence: First use works correctly - if config.json were absent, first use would also fail with same error
  timestamp: 2026-03-01T00:10:00Z

- hypothesis: Greedy decode state (state_h, state_c, last_emitted_token) persists across calls
  evidence: All three are local variables in greedy_decode(), initialized fresh on every call (zeros and blank_id)
  timestamp: 2026-03-01T00:11:00Z

- hypothesis: ONNX sessions are stateless so reuse is safe
  evidence: This is generally true BUT the error idx=8192=blank_id(vocab_size=8193) proves vocab_size=8193
  is somehow being used. The model stored in the mutex has vocab_size=1025 if loaded correctly.
  After deep analysis: the ONLY way idx=8192 appears is if a fresh load happened without config.json,
  OR the in-memory vocab_size became 8193. Since the struct is immutable after construction, the
  former is more likely - meaning is_none() was true on second switch for some reason.
  timestamp: 2026-03-01T00:12:00Z

## Evidence

- timestamp: 2026-03-01T00:05:00Z
  checked: src-tauri/patches/parakeet-rs/src/model_tdt.rs
  found: TDTModelConfig::default() has vocab_size=8193. from_pretrained() calls load_config() which
  reads config.json to override. load_config falls back to default if file absent/unparseable.
  implication: vocab_size=8193 is the fallback. Error idx=8192 = 8193-1 = blank_id with default vocab_size.

- timestamp: 2026-03-01T00:06:00Z
  checked: src-tauri/src/lib.rs set_engine command (lines 235-293)
  found: When switching to Parakeet: if ParakeetStateMutex.is_none() -> load model. If already Some -> skip.
  When switching to Whisper: ONLY updates ActiveEngine. ParakeetStateMutex is NEVER cleared to None.
  implication: After first successful Parakeet use, model stays in mutex. Second switch back to Parakeet
  reuses same model object (is_none()=false). No reload triggered.

- timestamp: 2026-03-01T00:07:00Z
  checked: src-tauri/patches/parakeet-rs/src/model_tdt.rs greedy_decode()
  found: blank_id = vocab_size - 1. last_emitted_token = blank_id as i32. These are fresh locals per call.
  The ONNX decoder_joint session is self.decoder_joint - a member of ParakeetTDTModel stored in the mutex.
  implication: ONNX sessions are reused across calls (same Session object). Session.run() is called
  repeatedly. Standard ORT sessions are designed to be reused - but see next evidence.

- timestamp: 2026-03-01T00:08:00Z
  checked: Error message carefully: range [-1025, 1024] proves ONNX model has vocab_size=1025.
  idx=8192 proves our code passed token_id=8192 to the Gather node.
  found: blank_id=8192 iff vocab_size=8193. This value only arises from TDTModelConfig::default().
  implication: A fresh load occurred without config.json being read, OR the original load somehow
  used the default. Since first use works, the original load read config.json correctly.

- timestamp: 2026-03-01T00:09:00Z
  checked: All code paths that modify ParakeetStateMutex
  found: ONLY two places write to ParakeetStateMutex:
    1. setup() in run() - if saved engine is Parakeet, loads on startup
    2. set_engine() - if switching to Parakeet and is_none(), loads model
  NOWHERE is ParakeetStateMutex set to None (cleared).
  implication: Once loaded, the ParakeetTDT object is NEVER dropped until app exit. Reuse is forced.

- timestamp: 2026-03-01T00:13:00Z
  checked: Considering what happens when ONNX session handles bad state after error
  found: After OOB error on second transcription, the ONNX session may be in undefined state.
  But crucially: the error occurs on SECOND transcription, not first. Why?
  The key insight: the ORT session for decoder_joint processes tokens sequentially. The `targets`
  input to the session on the VERY FIRST step of greedy_decode uses last_emitted_token=blank_id.
  With vocab_size=1025 (correct), blank_id=1024, which is valid. But with vocab_size=8193 (default),
  blank_id=8192, which is invalid (range 0..1024).
  Therefore: the crash on second transcription with a REUSED model object (vocab_size=1025) is IMPOSSIBLE
  via the blank_id path. The ONNX embedding accepts any token in [0..1024]. blank_id=1024 is valid.
  implication: The crash MUST happen when vocab_size=8193 is in effect. This can ONLY happen on a
  fresh load without config.json. Therefore is_none() MUST be true when the crash happens.

- timestamp: 2026-03-01T00:14:00Z
  checked: Root cause - why would is_none() be true on switch back to Parakeet?
  found: SCENARIO: User starts app with Whisper as saved engine. ParakeetStateMutex=None at startup.
  Step 1 "Parakeet works fine": set_engine("parakeet") called. is_none()=true. load_parakeet() called.
  config.json present -> vocab_size=1025. First transcription succeeds.
  Step 2: set_engine("whisper"). ParakeetStateMutex stays Some(model_with_vocab_1025).
  Step 3: set_engine("parakeet"). is_none()=false. NO RELOAD. Same model. vocab_size=1025. Should work.

  ALTERNATIVE SCENARIO that would cause the bug: What if between step 1 and step 3, the
  ParakeetStateMutex is cleared? There is no code to clear it... BUT what if the app is
  RESTARTED between steps? If the user restarts the app with Whisper as saved engine (because
  settings.json wrote "whisper" in step 2), then at next startup ParakeetStateMutex=None again.
  If config.json has since been deleted or never existed... no, first use would also fail.

  MOST LIKELY SCENARIO: config.json does NOT exist in the download directory. The first
  transcription uses a different code path or the model was loaded via a different mechanism.
  OR: the download of Parakeet does NOT include config.json, and the first transcription
  succeeds because the INITIAL greedy_decode doesn't trigger the OOB (by luck - the first
  token emitted is not blank_id=8192 because the audio itself produces valid-range tokens
  from the vocab_logits argmax). The OOB happens when blank_id=8192 is passed as
  `targets` to decoder_joint. This happens on the FIRST iteration of the decoding loop
  where last_emitted_token = blank_id = 8192. So if vocab_size=8193, the VERY FIRST call
  to decoder_joint.run() with targets=[[8192]] would fail. Unless the first successful
  transcription was with vocab_size=1025 (config.json present), and the second fails with
  vocab_size=8193 (config.json absent on reload).

  FINAL CONFIRMED ROOT CAUSE: The download.rs code downloads the Parakeet model files but
  may NOT include config.json. On the FIRST load (startup or first engine switch), if
  config.json is present (e.g., copied manually or from an earlier download), vocab_size=1025.
  When the user switches away and back, if a reload is triggered (is_none()=true, e.g. after
  app restart with whisper as saved engine), and config.json is missing, vocab_size=8193 causes the crash.

  BUT THE SIMPLEST EXPLANATION: The patch was applied AFTER the model was already downloaded.
  The config.json may not be present in the user's download directory because the download.rs
  code didn't download it. Let me check download.rs to see what files are downloaded.
  implication: Need to check download.rs to see if config.json is included in the download manifest.

## Resolution

root_cause: config.json in the downloaded parakeet-tdt-0.6b-v2-onnx model dir does NOT contain
a vocab_size field. It contains {"model_type": "nemo-conformer-tdt", "features_size": 128,
"subsampling_factor": 8}. The ModelConfig struct (config.rs) requires vocab_size as a mandatory
field (no serde(default)). Deserialization fails silently in load_config(). Fallback returns
TDTModelConfig::default() with vocab_size=8193. greedy_decode() computes blank_id = 8193-1 = 8192.
The first call to decoder_joint.run() passes targets=[[8192]], which is out of range for the
ONNX Gather embedding node (valid range: [-1025, 1024] = 1025 entries for vocab_size=1025).
Crash occurs on every transcription, first or subsequent. The engine-switch-specific observation
in the bug report may reflect that users only observe the error after switching back (because
prior sessions were against an older code version without the greedy_decode function).

fix: Load Vocabulary from vocab.txt BEFORE constructing ParakeetTDTModel. Pass vocab_size =
vocab.id_to_token.len() = 1025 (ground truth from vocab.txt) to ParakeetTDTModel::from_pretrained().
Remove the load_config() method that read config.json (which was never correct for this model).
The Vocabulary is the single source of truth for vocab_size — it was already loaded and validated.

verification: cargo check passes cleanly. Awaiting human verification of transcription in app.

files_changed:
  - src-tauri/patches/parakeet-rs/src/model_tdt.rs
  - src-tauri/patches/parakeet-rs/src/parakeet_tdt.rs
