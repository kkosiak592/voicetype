---
status: awaiting_human_verify
trigger: "model-selector-download-broken"
created: 2026-03-01T00:00:00Z
updated: 2026-03-01T00:01:00Z
---

## Current Focus

hypothesis: Download button is a nested <button> inside a disabled <button>, blocking all clicks
test: Read ModelSelector.tsx structure — confirmed nesting and disabled propagation
expecting: Replacing outer <button> with a <div> for non-downloaded models will allow inner Download button clicks
next_action: Replace outer <button> with <div> for the model card container

## Symptoms

expected: Clicking Download on a non-downloaded model starts a download with a progress bar
actual: Nothing happens when clicking the Download button — no visual change, no download initiated
errors: None reported
reproduction: Open settings → Model tab → click Download on a non-downloaded model
started: First test of this feature (phase 07-02). FirstRun.tsx works correctly.

## Eliminated

- hypothesis: onDownload prop not passed from ModelSection to ModelSelector
  evidence: ModelSection.tsx line 53 passes onDownloadComplete={handleDownloadComplete} correctly
  timestamp: 2026-03-01T00:00:00Z

- hypothesis: Channel<DownloadEvent> or invoke call wired incorrectly
  evidence: ModelSelector.tsx handleDownload() is identical in structure to the working FirstRun.tsx implementation
  timestamp: 2026-03-01T00:00:00Z

## Evidence

- timestamp: 2026-03-01T00:00:00Z
  checked: ModelSelector.tsx lines 109-158 — outer/inner button structure
  found: |
    Line 109: outer <button disabled={disabled}> — disabled evaluates to true when !model.downloaded (line 105)
    Line 141: inner <button onClick={handleDownload}> — nested inside the disabled outer button
    This is invalid HTML (button inside button). Browser either ignores the inner button entirely
    or the disabled state on the outer button prevents any pointer events from reaching the inner button.
    The e.stopPropagation() call is irrelevant because the click never fires to begin with.
  implication: The Download button can never be clicked because it is inside a disabled parent button

## Resolution

root_cause: |
  The Download button (line 141) is a <button> nested inside a disabled outer <button> (line 109).
  For non-downloaded models, disabled={!model.downloaded} = true on the outer button.
  HTML spec forbids button-in-button nesting; browsers suppress all descendant interactive events
  when a button is disabled. The inner Download button's onClick handler never fires.

fix: |
  Replace the outer <button> with a <div> for the model card. The outer element only needs to be
  a button for keyboard-accessible selection of already-downloaded models. For non-downloaded
  models it has no clickable purpose (cursor-default, disabled anyway). Use a <div> as the card
  container and put the selection logic only where it applies.

  Approach: Change the outer <button> to a <div> that conditionally attaches onClick only when
  the model is downloaded, or split into two render paths. The simplest minimal fix is to change
  the outer element tag from <button> to <div> and replicate the click/keyboard behavior explicitly.

verification: |
  Fix applied. Outer <button> replaced with <div>. Download <button> is no longer nested inside
  a disabled button element. The div handles model selection for downloaded models via onClick/onKeyDown
  with role="button" only when appropriate. Awaiting human verification in the running app.
files_changed:
  - src/components/ModelSelector.tsx
