---
phase: quick-42
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/components/sections/SystemSection.tsx
  - src/components/sections/ModelSection.tsx
  - src/components/Sidebar.tsx
  - src/App.tsx
autonomous: true
requirements: [QUICK-42]
must_haves:
  truths:
    - "System tab appears in sidebar between Appearance and History"
    - "System tab shows Inference Status card with GPU, Provider, and Engine"
    - "Model tab no longer shows the Inference Status card"
  artifacts:
    - path: "src/components/sections/SystemSection.tsx"
      provides: "System settings section with inference status"
    - path: "src/components/Sidebar.tsx"
      provides: "Updated sidebar with system entry"
  key_links:
    - from: "src/App.tsx"
      to: "src/components/sections/SystemSection.tsx"
      via: "activeSection === 'system' conditional render"
      pattern: "activeSection === 'system'"
---

<objective>
Add a "System" settings tab to the sidebar and move the Inference Status card from the Model section into it.

Purpose: Separate system/hardware info from model selection for cleaner settings organization.
Output: New SystemSection component, updated Sidebar and App routing, cleaned ModelSection.
</objective>

<context>
@src/components/Sidebar.tsx
@src/components/sections/ModelSection.tsx
@src/App.tsx
</context>

<tasks>

<task type="auto">
  <name>Task 1: Create SystemSection and move Inference Status out of ModelSection</name>
  <files>src/components/sections/SystemSection.tsx, src/components/sections/ModelSection.tsx</files>
  <action>
1. Create `src/components/sections/SystemSection.tsx`:
   - Import `useEffect, useState` from react, `invoke` from `@tauri-apps/api/core`
   - Copy the `GpuInfo` interface from ModelSection (then remove it from ModelSection)
   - Export `SystemSection` component (no props needed)
   - Inside: `useState<GpuInfo | null>(null)` for gpuInfo
   - `useEffect` on mount: `invoke<GpuInfo>('get_gpu_info').then(setGpuInfo).catch(console.error)`
   - Render with the same section header pattern as other sections:
     ```
     <h1> "System" </h1>
     <p> "Hardware and runtime information." </p>
     ```
   - Render the Inference Status card exactly as it appears in ModelSection (the `gpuInfo &&` block with the 3-column grid showing GPU, Provider, Engine) — copy the JSX verbatim
   - Additionally add an `AutostartToggle` import and render it below the inference status card inside its own card container (same ring/rounded-2xl/shadow-sm pattern). Move the AutostartToggle out of GeneralSection if it's there, or just add it as a second item in System.

   Actually, keep it simple: just the Inference Status card for now. No AutostartToggle move.

2. Update `src/components/sections/ModelSection.tsx`:
   - Remove the `GpuInfo` interface definition
   - Remove the `gpuInfo` useState and the useEffect that calls `get_gpu_info`
   - Remove the entire `{gpuInfo && (...)}` JSX block (the Inference Status card, lines ~190-209)
   - Remove the `gpuInfo` import from the GpuInfo type if unused
   - Keep everything else (ModelSelector, download handlers, engine warning) unchanged
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && npx tsc --noEmit 2>&1 | head -20</automated>
  </verify>
  <done>SystemSection.tsx exists with Inference Status card. ModelSection.tsx no longer renders inference status.</done>
</task>

<task type="auto">
  <name>Task 2: Wire System tab into Sidebar and App routing</name>
  <files>src/components/Sidebar.tsx, src/App.tsx</files>
  <action>
1. Update `src/components/Sidebar.tsx`:
   - Add `Monitor` (or `HardDrive`) to the lucide-react import (represents system/hardware)
   - Add `'system'` to the `SectionId` union type: `'general' | 'model' | 'microphone' | 'appearance' | 'system' | 'history'`
   - Add system entry to ITEMS array between appearance and history:
     `{ id: 'system', label: 'System', icon: Monitor }`
   - The ITEMS array order should be: general, model, microphone, appearance, system, history

2. Update `src/App.tsx`:
   - Add import: `import { SystemSection } from './components/sections/SystemSection';`
   - Add routing in the AnimatePresence section, after the appearance conditional:
     `{activeSection === 'system' && <SystemSection />}`
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && npx tsc --noEmit 2>&1 | head -20</automated>
  </verify>
  <done>System tab appears in sidebar. Clicking it renders SystemSection with inference status card. Model tab no longer shows inference status.</done>
</task>

</tasks>

<verification>
- `npx tsc --noEmit` passes with no errors
- App builds successfully: `npm run build`
</verification>

<success_criteria>
- System tab visible in sidebar with Monitor icon between Appearance and History
- Clicking System shows inference status (GPU name, execution provider, active engine)
- Model tab shows only model selector and download options, no inference status card
- No TypeScript errors
</success_criteria>
