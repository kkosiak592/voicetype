---
phase: quick-39
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/components/AllCapsToggle.tsx
  - src/components/sections/GeneralSection.tsx
autonomous: true
requirements: [QUICK-39]

must_haves:
  truths:
    - "ALL CAPS toggle is visible in General Settings"
    - "Toggling it calls set_all_caps on the backend"
    - "Toggle reflects the current backend state on load"
  artifacts:
    - path: "src/components/AllCapsToggle.tsx"
      provides: "Self-contained toggle that reads/writes all_caps via Tauri IPC"
    - path: "src/components/sections/GeneralSection.tsx"
      provides: "Renders AllCapsToggle in a new Output card"
  key_links:
    - from: "src/components/AllCapsToggle.tsx"
      to: "invoke('get_all_caps') / invoke('set_all_caps')"
      via: "@tauri-apps/api/core invoke"
    - from: "src/components/sections/GeneralSection.tsx"
      to: "src/components/AllCapsToggle.tsx"
      via: "import and render"
---

<objective>
Add an ALL CAPS toggle to the General Settings section. The backend commands already exist (`get_all_caps`, `set_all_caps`). This plan wires a frontend toggle to those commands, following the same pattern as `AutostartToggle`.

Purpose: Exposes the already-functional ALL CAPS pipeline feature to users.
Output: `AllCapsToggle.tsx` component + updated `GeneralSection.tsx` with a new Output card.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
</execution_context>

<context>
@src/components/AutostartToggle.tsx
@src/components/sections/GeneralSection.tsx
@src/components/sections/AppearanceSection.tsx
</context>

<tasks>

<task type="auto">
  <name>Task 1: Create AllCapsToggle component</name>
  <files>src/components/AllCapsToggle.tsx</files>
  <action>
    Create `src/components/AllCapsToggle.tsx` modelled exactly on `AutostartToggle.tsx`.

    - On mount, call `invoke<boolean>('get_all_caps')` to load the current state, set it as the initial `enabled` value. Show a pulse skeleton while loading (same as AutostartToggle).
    - On click, toggle `enabled`. Call `invoke('set_all_caps', { enabled: next })`. Update local state.
    - No store persistence needed — the backend (profiles.rs) is the source of truth for this setting.
    - Use the same emerald/gray toggle button styling as `AutostartToggle`.
    - `aria-checked`, `role="switch"`, `<span className="sr-only">Toggle ALL CAPS</span>`.
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && npx tsc --noEmit 2>&1 | head -30</automated>
  </verify>
  <done>File exists, no TypeScript errors, component matches AutostartToggle structure.</done>
</task>

<task type="auto">
  <name>Task 2: Add ALL CAPS card to GeneralSection</name>
  <files>src/components/sections/GeneralSection.tsx</files>
  <action>
    Import `AllCapsToggle` from `../AllCapsToggle`.

    After the existing Activation card (`space-y-4` div), add a second card with the same `bg-white dark:bg-gray-900 ring-1 ring-gray-200 dark:ring-gray-800 rounded-2xl p-4 shadow-sm` styling.

    Card content:
    ```tsx
    <section>
      <h2 className="text-sm font-semibold text-gray-900 dark:text-gray-100 mb-3">
        Output
      </h2>
      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm font-medium text-gray-900 dark:text-gray-100">ALL CAPS</p>
          <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
            Convert all transcribed text to uppercase
          </p>
        </div>
        <AllCapsToggle />
      </div>
    </section>
    ```

    No props changes to `GeneralSection` are needed — `AllCapsToggle` is self-contained.
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && npx tsc --noEmit 2>&1 | head -30</automated>
  </verify>
  <done>TypeScript clean. AllCapsToggle renders in General Settings below the Activation card.</done>
</task>

</tasks>

<verification>
Run `npx tsc --noEmit` — zero errors.
Launch the app, open General Settings, confirm the Output card and ALL CAPS toggle are visible.
Toggle it on, dictate text — output should be uppercase. Toggle off — output returns to normal case.
</verification>

<success_criteria>
- `src/components/AllCapsToggle.tsx` exists and compiles cleanly
- General Settings shows an "Output" card with an ALL CAPS toggle row
- Toggle reads current state from `get_all_caps` on mount
- Clicking toggle calls `set_all_caps` with the new boolean
</success_criteria>

<output>
After completion, create `.planning/quick/39-add-all-caps-toggle-to-the-frontend-ui/39-SUMMARY.md`
</output>
