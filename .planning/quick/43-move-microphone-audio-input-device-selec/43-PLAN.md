---
phase: quick-43
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/components/sections/SystemSection.tsx
  - src/components/Sidebar.tsx
  - src/App.tsx
  - src/components/sections/MicrophoneSection.tsx
autonomous: true
requirements: [QUICK-43]
must_haves:
  truths:
    - "Audio input device selector appears in the System settings tab"
    - "Microphone sidebar item is removed"
    - "Changing input device in System tab still persists and applies"
  artifacts:
    - path: "src/components/sections/SystemSection.tsx"
      provides: "Combined system info and mic selector"
      contains: "list_input_devices"
    - path: "src/components/Sidebar.tsx"
      provides: "Sidebar without Microphone entry"
  key_links:
    - from: "src/components/sections/SystemSection.tsx"
      to: "invoke('set_microphone')"
      via: "IPC call on select change"
      pattern: "set_microphone"
---

<objective>
Move the microphone/audio input device selector from its own dedicated settings tab into the System settings tab, alongside the inference status card created in quick-42. Remove the Microphone sidebar entry.

Purpose: Consolidate hardware-related settings (GPU info + mic selection) under one System tab instead of having mic as a separate top-level section.
Output: SystemSection.tsx with both inference status and mic selector; Microphone sidebar item removed.
</objective>

<execution_context>
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/kkosiak.TITANPC/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src/components/sections/SystemSection.tsx
@src/components/sections/MicrophoneSection.tsx
@src/components/Sidebar.tsx
@src/App.tsx
</context>

<interfaces>
<!-- From SystemSection.tsx -->
```typescript
export function SystemSection(): JSX.Element
// Currently takes no props, renders GPU info card
```

<!-- From MicrophoneSection.tsx -->
```typescript
interface MicrophoneSectionProps {
  selectedMic: string;
  onSelectedMicChange: (device: string) => void;
}
export function MicrophoneSection({ selectedMic, onSelectedMicChange }: MicrophoneSectionProps): JSX.Element
// Uses invoke('list_input_devices'), invoke('set_microphone'), getStore()
```

<!-- From Sidebar.tsx -->
```typescript
export type SectionId = 'general' | 'model' | 'microphone' | 'appearance' | 'system' | 'history';
// ITEMS array includes { id: 'microphone', label: 'Microphone', icon: Mic }
```

<!-- From App.tsx usage -->
```typescript
// selectedMic state + setSelectedMic setter passed to MicrophoneSection
// activeSection === 'microphone' renders MicrophoneSection
// activeSection === 'system' renders <SystemSection /> (no props)
```
</interfaces>

<tasks>

<task type="auto">
  <name>Task 1: Merge mic selector into SystemSection and update wiring</name>
  <files>src/components/sections/SystemSection.tsx, src/components/Sidebar.tsx, src/App.tsx, src/components/sections/MicrophoneSection.tsx</files>
  <action>
1. **SystemSection.tsx** — Add props `selectedMic: string` and `onSelectedMicChange: (device: string) => void`. Absorb the mic selection logic from MicrophoneSection directly into SystemSection:
   - Add `devices` state (`string[]`) and `loading` state, same useEffect calling `invoke('list_input_devices')`
   - Add `handleDeviceChange` function calling `invoke('set_microphone')` + store persistence (import `getStore` from `../../lib/store`)
   - Below the existing Inference Status card (inside the `space-y-4` div), add a new card with matching styling (same `bg-white dark:bg-gray-900 ring-1 ring-gray-200 dark:ring-gray-800 rounded-2xl p-4 shadow-sm` pattern) containing:
     - h2 "Input Device" (same style as "Inference Status" h2)
     - The select dropdown and helper text from MicrophoneSection (same Tailwind classes)
   - The card should render even while loading (show skeleton pulse for the select)

2. **Sidebar.tsx** — Remove `'microphone'` from the `SectionId` union type. Remove the `{ id: 'microphone', label: 'Microphone', icon: Mic }` entry from the ITEMS array. Remove the `Mic` import from lucide-react (only if Mic is no longer used elsewhere in the file — check: Mic is also used in the logo div, so keep the import).

3. **App.tsx** — Remove the `MicrophoneSection` import. Remove the `{activeSection === 'microphone' && ...}` block. Pass `selectedMic` and `onSelectedMicChange={setSelectedMic}` as props to `<SystemSection />`.

4. **MicrophoneSection.tsx** — Delete this file entirely. It is fully absorbed into SystemSection.
  </action>
  <verify>
    <automated>cd C:/Users/kkosiak.TITANPC/Desktop/Code/voice-to-text && npx tsc --noEmit 2>&1 | head -20</automated>
  </verify>
  <done>Mic selector renders inside System tab card. Microphone sidebar item gone. No TypeScript errors. Selecting a device still calls set_microphone IPC and persists to store.</done>
</task>

</tasks>

<verification>
- `npx tsc --noEmit` passes with zero errors
- App compiles and runs: Sidebar shows General, Model, Appearance, System, History (no Microphone)
- System tab shows both Inference Status card and Input Device card
- Changing mic device in System tab persists selection
</verification>

<success_criteria>
- Microphone sidebar entry removed
- Audio input device selector renders in System tab below inference status
- Device selection still functional (IPC + store persistence)
- No TypeScript compilation errors
- MicrophoneSection.tsx deleted
</success_criteria>

<output>
After completion, create `.planning/quick/43-move-microphone-audio-input-device-selec/43-01-SUMMARY.md`
</output>
