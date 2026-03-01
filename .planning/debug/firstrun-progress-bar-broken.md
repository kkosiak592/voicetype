---
status: awaiting_human_verify
trigger: "firstrun-progress-bar-broken"
created: 2026-03-01T00:00:00Z
updated: 2026-03-01T00:00:01Z
---

## Current Focus

hypothesis: CONFIRMED. serde's rename_all on an enum container only renames variant discriminants, not struct variant fields. DownloadEvent was sending downloaded_bytes/total_bytes as snake_case but frontend expected downloadedBytes/totalBytes camelCase.
test: cargo check — passed cleanly.
expecting: Fix resolves progress bar stuck at 0%, bar now reflects real download progress.
next_action: Human verification — rebuild and test the download flow in the app.

## Symptoms

expected: Progress bar should gradually fill from 0% to 100% as download streams chunks
actual: Progress bar likely stuck or not updating visually during the download, then jumps to complete. Download itself works fine.
errors: No errors reported in console
reproduction: Open settings with no model files, trigger download from FirstRun UI
started: First time testing (just built in phase 07)

## Eliminated

(none yet)

## Evidence

- timestamp: 2026-03-01T00:00:00Z
  checked: download.rs DownloadEvent enum definition
  found: |
    #[derive(Clone, Serialize)]
    #[serde(rename_all = "camelCase", tag = "event", content = "data")]
    pub enum DownloadEvent {
        Started { url: String, total_bytes: u64 },
        Progress { downloaded_bytes: u64, total_bytes: u64 },
        Finished,
        Error { message: String },
    }
  implication: |
    The #[serde(rename_all = "camelCase")] on the enum renames the VARIANT NAMES:
    Started -> "started", Progress -> "progress", Finished -> "finished", Error -> "error".
    But the FIELD NAMES inside struct variants (total_bytes, downloaded_bytes) are NOT
    automatically renamed by the enum-level attribute in serde. They remain snake_case.

- timestamp: 2026-03-01T00:00:00Z
  checked: FirstRun.tsx DownloadEvent type and onmessage handler
  found: |
    type DownloadEvent =
      | { event: 'started'; data: { url: string; totalBytes: number } }
      | { event: 'progress'; data: { downloadedBytes: number; totalBytes: number } }
    onEvent.onmessage handler reads msg.data.downloadedBytes and msg.data.totalBytes
  implication: |
    Frontend expects camelCase field names in the data object.
    If backend sends snake_case, msg.data.downloadedBytes === undefined and
    msg.data.totalBytes === undefined. setDownloadedBytes(undefined) and
    setTotalBytes(undefined) -> NaN in JS. percent = NaN, renders as null (0% width).
    This explains exactly: bar never moves, download completes, then jumps to 'complete'.

- timestamp: 2026-03-01T00:00:00Z
  checked: Serde documentation for rename_all on adjacently-tagged enums
  found: |
    For serde with tag+content ("adjacently tagged"), rename_all on the enum level
    applies only to variant names, not to the fields within variants.
    To rename fields, you must annotate each variant's fields separately or
    use a named struct with its own rename_all attribute.
  implication: |
    Confirmed root cause. The fix is to add #[serde(rename_all = "camelCase")]
    to each struct variant, OR restructure to use named structs with their own derive.
    The cleanest fix: add field-level #[serde(rename = "...")] or add a struct
    annotation. In Rust enums, you can add per-variant serde attributes like
    #[serde(rename_all = "camelCase")] at the variant level — but this is NOT
    supported in standard serde for struct variants inside enums.
    The correct approach: use #[serde(rename = "camelCaseName")] on each field.

## Resolution

root_cause: |
  download.rs DownloadEvent has #[serde(rename_all = "camelCase")] on the enum,
  which in serde only renames variant discriminant strings (started/progress/etc),
  NOT the field names within struct variants. So downloaded_bytes and total_bytes
  are sent as snake_case JSON keys. The frontend reads camelCase (downloadedBytes,
  totalBytes) which are undefined, causing setDownloadedBytes(undefined) ->
  percent = NaN -> progress bar never renders a real width.

fix: |
  Add #[serde(rename = "downloadedBytes")] and #[serde(rename = "totalBytes")]
  and #[serde(rename = "totalBytes")] on the struct variant fields in DownloadEvent,
  AND #[serde(rename = "totalBytes")] for the Started variant's total_bytes field.
  Alternatively use serde_with or define named structs.
  Cleanest minimal fix: add per-field serde rename attributes.

verification: cargo check passes cleanly; human runtime verification pending
files_changed:
  - src-tauri/src/download.rs
