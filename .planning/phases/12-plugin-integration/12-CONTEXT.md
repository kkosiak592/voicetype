# Phase 12: Plugin Integration - Context

**Gathered:** 2026-03-02
**Status:** Ready for planning

<domain>
## Phase Boundary

The app checks for updates on launch and guides the user through download, install, and relaunch without blocking normal use. This phase adds tauri-plugin-updater and tauri-plugin-process, implements the update check flow, notification UI, download progress, and auto-relaunch. Creating the CI/CD pipeline and release workflow are separate phases (13, 14).

</domain>

<decisions>
## Implementation Decisions

### Update Notification
- Tray icon indicator (badge/dot or menu item) for passive awareness when user isn't in settings
- Banner at top of settings window for details and action when user opens settings
- Show version number and release notes summary (from GitHub Release body)
- Notification is dismissible but reappears on next settings open / next launch
- No "skip this version" feature — keep implementation simple

### Download & Progress UX
- Progress shown inline in the settings banner (banner transforms into progress view)
- Reuse the existing Channel-based progress pattern from FirstRun.tsx / download.rs
- Download is cancellable with a Cancel button (matches FirstRun pattern)
- On failure: show error message immediately with Retry button (matches FirstRun pattern, no auto-retry)
- Download runs in the Rust backend — continues even if settings window is closed
- When settings is reopened, UI reconnects to current download state

### Relaunch Behavior
- After download completes: show "Update ready — Restart now?" with Restart Now / Later buttons
- Check if user is actively recording/dictating before relaunching — defer until idle if active
- If user chooses "Later": update installs on next app restart, no additional nagging during session
- No option to disable automatic update checks (always on — keeps users on latest version)

### Update Check Timing
- Check on launch with a short delay (3-5s) so it doesn't compete with app startup
- Also check periodically (every few hours) for long-running sessions
- "Check for updates" button in General settings — shows "Up to date" or triggers update flow
- **Version number displayed in settings** (user-specified decision)

### Claude's Discretion
- Exact placement of version number in settings UI (footer, General section header, etc.)
- Periodic check interval (e.g., every 4h vs 6h vs 12h)
- Exact banner styling and animation
- Tray indicator implementation (dot overlay, different icon, menu item text)
- Loading/checking state UI
- Error state copy and styling

</decisions>

<specifics>
## Specific Ideas

- User wants version number visible somewhere in settings — not just buried in About/installer
- All other UX decisions deferred to standard desktop app updater practices

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `download.rs` DownloadEvent enum (started/progress/finished/error) with Channel-based streaming — exact same pattern needed for update download progress
- `FirstRun.tsx` progress bar UI with cancel/retry — reusable pattern for update download progress
- `tray.rs` — existing tray icon and context menu infrastructure for adding update indicator
- `Sidebar.tsx` — settings navigation, may need "Check for updates" in General section
- `GeneralSection.tsx` — likely home for version display and manual update check button

### Established Patterns
- Tauri Channel IPC for streaming progress events from Rust to React
- Tailwind CSS with indigo accent color for primary actions
- Settings organized into sidebar sections with `SectionId` type
- tauri-plugin-store for persisting settings (could store "update available" state)

### Integration Points
- `Cargo.toml` — add tauri-plugin-updater and tauri-plugin-process dependencies
- `lib.rs` — register new plugins on the Tauri Builder
- `tauri.conf.json` — updater config already present (pubkey + endpoint), needs permissions
- `capabilities/` — add updater:default and process:allow-restart permissions
- `tray.rs` — add update indicator to tray menu
- `GeneralSection.tsx` — add version display and "Check for updates" button
- Settings window — add update banner component (top of window, above sidebar content)

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 12-plugin-integration*
*Context gathered: 2026-03-02*
