<objective>
Completely redesign and revamp the UI of this Tauri voice-to-text desktop app ("VoiceType") to look and feel like a modern, premium desktop application. The current UI is functional but visually plain — basic Tailwind utility classes, no design system, unicode characters as icons, flat sections with simple separators. Transform it into something that feels polished, cohesive, and delightful to use.

This is a settings/configuration panel app (sidebar + content area layout) with a separate floating pill overlay window for recording status. The app uses React 18, Tailwind CSS v4, and TypeScript. Do NOT touch any Rust/backend code or change any Tauri invoke calls, event listeners, or state management logic. This is a purely visual redesign.
</objective>

<context>
Read the CLAUDE.md file first for project conventions.

The app has two windows:
1. **Main settings window** — sidebar navigation + content panels (General, Model, Microphone, Appearance, History)
2. **Pill overlay window** — small floating pill that shows recording/processing state (Pill.tsx, pill.css) — DO NOT redesign this, it already has a polished glass aesthetic

**Tech stack:**
- React 18 + TypeScript
- Tailwind CSS v4 (uses `@import "tailwindcss"` and `@variant dark` directive)
- Tauri v2 (desktop app, not web)
- No component library — all custom components
- Dark mode via `.dark` class on `<html>`

**Current file structure to modify:**
- `src/styles.css` — global styles (currently minimal)
- `src/App.tsx` — main layout, sidebar + content area
- `src/components/Sidebar.tsx` — navigation sidebar with unicode icons
- `src/components/sections/GeneralSection.tsx` — hotkey, recording mode, updates
- `src/components/sections/ModelSection.tsx` — model selection, GPU info, downloads
- `src/components/sections/MicrophoneSection.tsx` — device selector dropdown
- `src/components/sections/AppearanceSection.tsx` — theme toggle, autostart toggle
- `src/components/sections/HistorySection.tsx` — transcription history list
- `src/components/ModelSelector.tsx` — model cards with download progress
- `src/components/HotkeyCapture.tsx` — hotkey input capture box
- `src/components/RecordingModeToggle.tsx` — hold/toggle mode selector cards
- `src/components/ThemeToggle.tsx` — dark mode switch
- `src/components/UpdateBanner.tsx` — update notification banners
- `src/components/FirstRun.tsx` — first-run setup wizard
- `src/components/FrequencyBars.tsx` — DO NOT MODIFY (canvas-based animation)
- `src/components/ProcessingDots.tsx` — DO NOT MODIFY (pill animation)
- `src/pill.css` — DO NOT MODIFY
- `src/Pill.tsx` — DO NOT MODIFY
- `src/pill-main.tsx` — DO NOT MODIFY

**Do NOT modify these files** (they are pill window files with their own design):
- `src/Pill.tsx`, `src/pill.css`, `src/pill-main.tsx`
- `src/components/FrequencyBars.tsx`, `src/components/ProcessingDots.tsx`
</context>

<research>
Before making changes, read ALL of the following files to understand the current implementation, component interfaces, and state flow:

1. `src/App.tsx`
2. `src/styles.css`
3. `src/components/Sidebar.tsx`
4. `src/components/sections/GeneralSection.tsx`
5. `src/components/sections/ModelSection.tsx`
6. `src/components/sections/MicrophoneSection.tsx`
7. `src/components/sections/AppearanceSection.tsx`
8. `src/components/sections/HistorySection.tsx`
9. `src/components/ModelSelector.tsx`
10. `src/components/HotkeyCapture.tsx`
11. `src/components/RecordingModeToggle.tsx`
12. `src/components/ThemeToggle.tsx`
13. `src/components/AutostartToggle.tsx`
14. `src/components/UpdateBanner.tsx`
15. `src/components/FirstRun.tsx`
16. `src/components/DictionaryEditor.tsx`

Understand every component's props, state, and Tauri invoke calls before modifying anything. Every invoke(), listen(), Channel, and store interaction must remain exactly as-is.
</research>

<design_direction>
Thoroughly consider multiple modern desktop app design aesthetics and choose the best approach. Think about apps like Linear, Raycast, Arc Browser, Notion, and Figma for inspiration. The redesign should feel like a native desktop app, not a website.

**Design principles to follow:**

1. **Depth and layering** — Use subtle shadows, backdrop blur, layered surfaces with different opacities. The sidebar and content area should feel like distinct surfaces with depth between them.

2. **Refined color palette** — Move beyond basic gray/indigo. Create a sophisticated, cohesive color system:
   - Light mode: warm neutrals (not cold grays), subtle tinted backgrounds
   - Dark mode: rich, deep backgrounds with subtle color tinting (not pure black/gray)
   - Accent color: a distinctive brand accent (consider violet, teal, or a warm accent) used sparingly for active states, focus rings, and CTAs
   - Semantic colors for status: success (green), warning (amber), error (red), info (blue)

3. **Typography hierarchy** — Clean, tight typography with clear visual hierarchy. Section headers should be distinctive but not loud. Use font-weight and size contrast, not just color.

4. **Micro-interactions and polish** — Smooth transitions on hover/focus/active states. Subtle scale transforms on interactive cards. Animated state changes. Focus rings that feel intentional, not default.

5. **Proper iconography** — Replace ALL unicode character icons (⌨, ◎, ◉, ◐, ◷) with inline SVG icons. Use a consistent icon style (outlined, 20x20 or 24x24). Create clean, recognizable icons for: General/Settings (gear), Model (brain/cube), Microphone (mic), Appearance (palette/sun-moon), History (clock).

6. **Card-based layouts** — Group related settings into visually distinct cards/panels with rounded corners, subtle borders, and consistent padding. No bare `<hr>` separators — use card boundaries instead.

7. **Form controls** — Elevate all form elements:
   - Select dropdowns: custom styled, not browser-default `<select>`
   - Toggle switches: smoother, larger, with color transitions
   - Buttons: consistent sizing, hover states with subtle transforms
   - The hotkey capture box: more visually distinctive with a keyboard-key aesthetic
   - Radio/card selectors: clear selected vs unselected states with smooth transitions

8. **Empty states** — Design thoughtful empty states (e.g., History section with no entries) with subtle illustrations or icons, not just plain text.

9. **Loading states** — Skeleton loaders should feel cohesive with the design, not just gray boxes.

10. **Sidebar design** — The sidebar should feel premium:
    - Subtle active indicator (left accent bar or background highlight)
    - Icon + label alignment
    - Bottom section for version info
    - Hover states with smooth transitions
    - Consider a section divider or grouping

11. **Window chrome integration** — Since this is a Tauri desktop app, the content should feel integrated with the window. Consider data-tauri-drag-region on the top area for draggable title bar feel.

12. **Responsive within window** — The app window is fixed-size desktop, but content should handle varying amounts of data gracefully (model list, history entries, etc.).
</design_direction>

<requirements>
1. **Preserve ALL functionality** — Every button click, every toggle, every invoke() call, every event listener, every store operation must continue to work exactly as before. Do not change any component props interfaces, state management, or Tauri IPC calls.

2. **Preserve component structure** — Keep the same file structure and component hierarchy. Do not merge or split components. Do not rename files.

3. **Replace unicode icons with SVGs** — Create proper inline SVG icons for the sidebar and anywhere else unicode characters are used as icons. SVGs should be clean, consistent in style, and sized appropriately.

4. **Enhance `src/styles.css`** — Add CSS custom properties for the design system (colors, spacing, shadows, transitions). Add any global styles, custom component classes, or animations needed. Keep Tailwind as the primary utility system.

5. **Dark mode must work perfectly** — Every element must have proper dark mode variants. Dark mode should feel intentionally designed, not just inverted.

6. **Accessibility** — Maintain all existing ARIA attributes. Ensure sufficient color contrast in both modes. Keep focus indicators visible and styled.

7. **No new dependencies** — Do not install any new npm packages. Use only React, Tailwind CSS v4, and inline SVGs. No icon libraries, no animation libraries, no component libraries.

8. **FirstRun wizard** — Give the first-run experience special attention. It's the user's first impression. Make the model cards more visually appealing, the progress indicators more polished, and the overall flow feel welcoming.

9. **UpdateBanner** — Redesign all update banner states (checking, available, downloading, ready, error) to feel cohesive with the new design system while remaining clearly distinct from each other.

10. **History section** — Make history entries feel like a proper list with hover states, timestamps that are easy to scan, and a satisfying copy interaction.

11. **Model selector cards** — These are complex (selected state, download button, progress bar, error state, loading state). Each state needs clear visual treatment. The GPU info panel should feel like an informational sidebar, not an afterthought.
</requirements>

<constraints>
- Do NOT modify: `src/Pill.tsx`, `src/pill.css`, `src/pill-main.tsx`, `src/components/FrequencyBars.tsx`, `src/components/ProcessingDots.tsx`
- Do NOT modify any file in `src-tauri/` (Rust backend)
- Do NOT change any TypeScript interfaces, prop types, or component APIs
- Do NOT change any `invoke()`, `listen()`, `Channel`, or store calls
- Do NOT add new npm dependencies — everything must be done with existing packages
- Do NOT change the sidebar navigation IDs or section routing logic
- Do NOT change how dark mode is toggled (`.dark` class on `<html>`)
- Keep the same Tailwind CSS v4 setup (no switching to v3 or other frameworks)
</constraints>

<implementation>
Work through the redesign systematically:

1. **Start with `src/styles.css`** — Define the design system foundation: CSS custom properties for colors, shadows, border radii, transitions. Add any utility classes or component base styles needed.

2. **Redesign `Sidebar.tsx`** — Replace unicode icons with SVGs, add active indicator styling, version info at bottom, premium hover/active states.

3. **Update `App.tsx` layout** — Refine the overall layout with proper surfaces, spacing, and depth.

4. **Redesign each section** in order:
   - `GeneralSection.tsx` — Card-based subsections, refined hotkey capture, polished recording mode toggle
   - `ModelSection.tsx` + `ModelSelector.tsx` — Premium model cards, refined progress bars, polished GPU info panel
   - `MicrophoneSection.tsx` — Custom-styled select dropdown
   - `AppearanceSection.tsx` — Refined toggle switches, card layout
   - `HistorySection.tsx` — Polished list design with better empty state

5. **Redesign shared components:**
   - `HotkeyCapture.tsx` — Keyboard-key aesthetic
   - `RecordingModeToggle.tsx` — More polished card selection
   - `ThemeToggle.tsx` — Smoother, more premium toggle
   - `AutostartToggle.tsx` — Match ThemeToggle style
   - `UpdateBanner.tsx` — Cohesive banner designs for all states
   - `DictionaryEditor.tsx` — If it exists and is used

6. **Redesign `FirstRun.tsx`** — Premium first-run experience with better model cards and progress states.

7. **Final pass** — Review all components for consistency, transitions, and dark mode coverage.

For maximum efficiency, whenever you need to perform multiple independent operations (like reading files or checking patterns), invoke all relevant tools simultaneously rather than sequentially.

After receiving tool results, carefully reflect on their quality and determine optimal next steps before proceeding.
</implementation>

<verification>
After completing the redesign, verify your work:

1. Confirm every component still has the same props interface (no breaking changes)
2. Confirm every `invoke()`, `listen()`, `Channel`, and `store` call is unchanged
3. Confirm dark mode classes are present on every styled element
4. Confirm no unicode icons remain in the sidebar (all replaced with SVGs)
5. Confirm no `<hr>` elements remain (replaced with card boundaries or better separators)
6. Confirm `src/Pill.tsx`, `src/pill.css`, `src/pill-main.tsx`, `FrequencyBars.tsx`, and `ProcessingDots.tsx` are unmodified
7. Confirm no new npm dependencies were added
8. Run `npm run build` to verify TypeScript compilation succeeds
9. Run `npm run tauri dev` to visually verify the app loads correctly, sidebar navigation works, and all sections render properly in both light and dark mode
</verification>

<success_criteria>
- The app looks and feels like a premium modern desktop application (think Linear, Raycast quality)
- Light and dark modes are both intentionally designed and visually cohesive
- All icons are clean inline SVGs, no unicode characters used as icons
- Interactive elements have smooth hover/focus/active transitions
- Settings are organized in clear card-based groups
- The first-run wizard feels welcoming and polished
- All existing functionality works identically to before
- TypeScript compiles without errors
- The app runs correctly in Tauri
</success_criteria>
