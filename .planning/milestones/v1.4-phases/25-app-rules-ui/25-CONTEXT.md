# Phase 25: App Rules UI - Context

**Gathered:** 2026-03-07
**Status:** Ready for planning

<domain>
## Phase Boundary

New "App Rules" sidebar page for managing per-app ALL CAPS overrides. Users can detect the foreground app, view configured rules, change override state via dropdown, and remove rules. Process dropdown (Phase 26) is out of scope.

</domain>

<decisions>
## Implementation Decisions

### Three-state toggle UX
- Dropdown menu (not segmented control or cycling button)
- Three options: Inherit, Force ON, Force OFF
- "Inherit" option shows current global ALL CAPS state: e.g., "Inherit (OFF)" or "Inherit (ON)"
- New apps default to "Inherit" when added
- Force ON / Force OFF label style: Claude's discretion

### Detect flow
- Inline countdown on button: button itself shows "Detecting in 3... 2... 1..." then flashes success "Added acad.exe"
- No modal, toast, or confirmation step — everything happens in-place on the button
- Duplicate detection: Claude's discretion (show "already added" or scroll to existing)
- Failure state: button shows "Could not detect app — try again" for a few seconds, then resets
- After successful detect, app appears in rules list with Inherit default

### Rules list layout
- Two-line rows: exe name (bold) + window title subtitle
- Each row: app info on left, ALL CAPS dropdown + delete button (x) on right
- Delete immediately on click — no confirmation needed
- Dropdown color coding: Claude's discretion (color-coded by state vs neutral)
- Empty state: centered message "No app rules configured" with hint to use Detect button

### Page structure
- Sidebar position: after Dictionary (General > Dictionary > App Rules > Model > Appearance > System > History)
- Sidebar icon: Claude's discretion (lucide icon)
- Cross-reference from General page ALL CAPS toggle: Claude's discretion
- Page header: "App Rules" with subtitle describing per-app overrides and showing global default state

### Claude's Discretion
- Dropdown label style for Force ON / Force OFF
- Dropdown color coding (green/red/neutral vs uniform)
- Sidebar icon choice from lucide
- Whether to add cross-reference hint on General page
- Duplicate app detection UX behavior
- Framer-motion page transition (follow existing pattern)

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Sidebar.tsx`: `SectionId` type union + `ITEMS` array with lucide icons — add 'app-rules' to type and array
- `AllCapsToggle.tsx`: Existing boolean toggle on General page — remains as global default
- `GeneralSection.tsx`: Card pattern with `bg-white dark:bg-gray-900 ring-1 ring-gray-200 dark:ring-gray-800 rounded-2xl p-4 shadow-sm`
- Framer-motion `AnimatePresence` with `mode="wait"` for page transitions in App.tsx

### Established Patterns
- Section components in `src/components/sections/` — one file per sidebar page
- Tauri IPC via `invoke()` from `@tauri-apps/api/core`
- Dark mode via Tailwind `dark:` prefix throughout
- Emerald accent color for active/success states
- `twMerge` for conditional class composition

### Integration Points
- `Sidebar.tsx:7`: Add to `SectionId` type union
- `Sidebar.tsx:15-22`: Add to `ITEMS` array after Dictionary
- `App.tsx:195-223`: Add conditional render for new section
- Backend commands already exist: `detect_foreground_app`, `get_app_rules`, `set_app_rule`, `remove_app_rule`
- `store.ts`: May need to read global `all_caps` state to display in Inherit dropdown option

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches matching existing UI patterns.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 25-app-rules-ui*
*Context gathered: 2026-03-07*
