# Phase 25: App Rules UI - Research

**Researched:** 2026-03-07
**Domain:** React/Tauri settings UI - per-app rules management page
**Confidence:** HIGH

## Summary

This phase adds a new "App Rules" sidebar page for managing per-app ALL CAPS overrides. The backend infrastructure is fully complete: `detect_foreground_app`, `get_app_rules`, `set_app_rule`, and `remove_app_rule` Tauri commands are already registered and functional (lib.rs lines 1143-1190, registered at lines 1865-1871). The `AppRule` struct uses `Option<bool>` for three-state logic (None=inherit, Some(true)=force ON, Some(false)=force OFF).

The work is purely frontend: a new `AppRulesSection.tsx` component following the exact patterns established by existing sections (DictionarySection, GeneralSection), sidebar registration in `Sidebar.tsx` and `App.tsx`, and the detect-app flow with countdown timer.

**Primary recommendation:** Build `AppRulesSection.tsx` as a self-contained section component matching existing card/header patterns, using direct `invoke()` calls to the four existing backend commands. No new libraries needed.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Three-state toggle: Dropdown menu (not segmented control or cycling button)
- Three options: Inherit, Force ON, Force OFF
- "Inherit" shows current global ALL CAPS state: e.g., "Inherit (OFF)" or "Inherit (ON)"
- New apps default to "Inherit" when added
- Detect flow: Inline countdown on button itself "Detecting in 3... 2... 1..." then flash success "Added acad.exe"
- No modal, toast, or confirmation step -- everything in-place on the button
- Failure state: button shows "Could not detect app -- try again" for a few seconds, then resets
- Rules list: Two-line rows -- exe name (bold) + window title subtitle
- Each row: app info on left, ALL CAPS dropdown + delete button (x) on right
- Delete immediately on click -- no confirmation needed
- Empty state: centered message "No app rules configured" with hint to use Detect button
- Sidebar position: after Dictionary (General > Dictionary > App Rules > Model > Appearance > System > History)
- Page header: "App Rules" with subtitle describing per-app overrides and showing global default state

### Claude's Discretion
- Dropdown label style for Force ON / Force OFF
- Dropdown color coding (green/red/neutral vs uniform)
- Sidebar icon choice from lucide
- Whether to add cross-reference hint on General page
- Duplicate app detection UX behavior
- Framer-motion page transition (follow existing pattern)

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| UI-01 | New "App Rules" sidebar page accessible from navigation | Add 'app-rules' to SectionId type union and ITEMS array in Sidebar.tsx; add conditional render in App.tsx |
| UI-02 | User can view a list of configured per-app rules with app icons and names | `get_app_rules` command returns HashMap<String, AppRule>; display as two-line rows with exe name + window title |
| UI-03 | User can add an app via "Detect Active App" button with 3-second countdown | `detect_foreground_app` command returns DetectedApp with exe_name and window_title; countdown via setInterval |
| UI-05 | User can remove an app from the rules list | `remove_app_rule` command takes exe_name; immediate delete, no confirmation |
| OVR-01 | Each app rule has a three-state ALL CAPS toggle (Inherit / Force ON / Force OFF) | `set_app_rule` command takes exe_name + AppRule with all_caps: Option<bool>; dropdown menu UX |
</phase_requirements>

## Standard Stack

### Core (already in project)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| React | ^18.3.1 | UI framework | Project standard |
| @tauri-apps/api | ^2 | IPC via invoke() | Project standard |
| lucide-react | ^0.577.0 | Sidebar and UI icons | Already used for all sidebar icons |
| tailwind-merge | ^3.5.0 | Conditional class composition | Used throughout via twMerge |
| framer-motion | ^12.35.0 | Page transitions | AnimatePresence in App.tsx |
| Tailwind CSS | ^4 | Styling with dark: prefix | Project standard |

### Supporting
No additional libraries needed. The dropdown can be built with native HTML `<select>` or a custom dropdown using existing Tailwind patterns. A custom dropdown is preferred for visual consistency with the rest of the UI.

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Custom dropdown | Native `<select>` | Native select is simpler but cannot be color-coded or styled consistently across platforms |
| Custom dropdown | Headless UI / Radix | Adds dependency for a single dropdown; project has zero headless UI libraries |

**Installation:** No new packages required.

## Architecture Patterns

### Recommended Project Structure
```
src/components/sections/
  AppRulesSection.tsx      # New page component (main deliverable)
src/components/
  AppRuleRow.tsx           # Optional: extracted row component if section gets large
```

### Pattern 1: Section Component Structure
**What:** Each sidebar page is a single component in `src/components/sections/` following the same header + card layout.
**When to use:** Always for new pages.
**Example (from GeneralSection.tsx and DictionarySection.tsx):**
```typescript
export function AppRulesSection() {
  // State + useEffect for data loading
  return (
    <div>
      {/* Header: h1 + subtitle p */}
      <div className="mb-4">
        <h1 className="text-xl font-bold tracking-tight text-gray-900 dark:text-gray-100">
          App Rules
        </h1>
        <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
          Per-app overrides subtitle...
        </p>
      </div>

      {/* Card container */}
      <div className="bg-white dark:bg-gray-900 ring-1 ring-gray-200 dark:ring-gray-800 rounded-2xl p-4 shadow-sm">
        {/* Content */}
      </div>
    </div>
  );
}
```

### Pattern 2: Tauri IPC for CRUD Operations
**What:** Direct invoke() calls with optimistic local state updates.
**When to use:** All backend interactions in this phase.
**Example (from AllCapsToggle.tsx and DictionarySection.tsx):**
```typescript
// Load on mount
useEffect(() => {
  invoke<Record<string, AppRule>>('get_app_rules')
    .then(setRules)
    .catch(err => console.error('Failed to load app rules:', err));
}, []);

// Set/update rule
async function handleSetRule(exeName: string, allCaps: boolean | null) {
  const rule = { all_caps: allCaps };
  await invoke('set_app_rule', { exeName, rule });
  setRules(prev => ({ ...prev, [exeName]: rule }));
}

// Remove rule
async function handleRemove(exeName: string) {
  await invoke('remove_app_rule', { exeName });
  setRules(prev => {
    const next = { ...prev };
    delete next[exeName];
    return next;
  });
}
```

### Pattern 3: Sidebar Registration
**What:** Add new page to SectionId type union and ITEMS array.
**Integration points:**
```typescript
// Sidebar.tsx line 7 - add to union type
export type SectionId = 'general' | 'dictionary' | 'app-rules' | 'model' | 'appearance' | 'system' | 'history';

// Sidebar.tsx line 15-22 - add after dictionary entry
{ id: 'app-rules', label: 'App Rules', icon: AppWindow }, // or similar lucide icon

// App.tsx - add import and conditional render
{activeSection === 'app-rules' && <AppRulesSection />}
```

### Pattern 4: Detect Flow with Countdown
**What:** Button shows inline countdown, detects foreground app, shows result.
**Implementation approach:**
```typescript
type DetectState = 'idle' | 'countdown' | 'success' | 'error';

function useDetectFlow(onDetected: (app: DetectedApp) => void) {
  const [state, setState] = useState<DetectState>('idle');
  const [countdown, setCountdown] = useState(3);
  const [message, setMessage] = useState('');

  async function startDetect() {
    setState('countdown');
    // Count down 3..2..1 using setInterval
    // After countdown hits 0, call invoke('detect_foreground_app')
    // On success: setState('success'), setMessage('Added exe.name')
    // On failure: setState('error'), setMessage('Could not detect app')
    // After 2-3 seconds, reset to 'idle'
  }
  return { state, countdown, message, startDetect };
}
```

### Anti-Patterns to Avoid
- **Polling for foreground app:** Detection only happens on button click, not continuously.
- **Storing window_title in AppRule:** The backend AppRule struct only has all_caps. Window titles are transient display data from the detect call, not persisted.
- **Using native `<select>` for three-state dropdown:** Cannot be styled/color-coded consistently. Build custom dropdown with click-outside-to-close.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Backend CRUD for app rules | Custom IPC protocol | Existing invoke() commands | `get_app_rules`, `set_app_rule`, `remove_app_rule`, `detect_foreground_app` already registered |
| Settings persistence | Custom file I/O | Existing `set_app_rule` / `remove_app_rule` | Backend handles settings.json flush automatically |
| Foreground app detection | Win32 calls from frontend | `detect_foreground_app` command | Already implemented in foreground.rs with UWP resolution |
| Page transitions | Custom animation | Existing framer-motion AnimatePresence in App.tsx | Already wraps all section renders with fade + slide |

**Key insight:** The entire backend is done. This phase is 100% frontend work using existing Tauri commands.

## Common Pitfalls

### Pitfall 1: Window Title Not Persisted
**What goes wrong:** Trying to store/retrieve window_title alongside the app rule.
**Why it happens:** The backend `AppRule` struct only contains `all_caps: Option<bool>`. Window title comes from `DetectedApp` at detection time.
**How to avoid:** Store window titles in local React state only (from the detect call). When loading rules on mount via `get_app_rules`, only exe names are available -- window titles will be blank until re-detected.
**Warning signs:** Trying to add window_title to the set_app_rule payload.

### Pitfall 2: Countdown Timer Cleanup
**What goes wrong:** setInterval leaks if component unmounts during countdown or user navigates away.
**Why it happens:** React strict mode double-mounts, or user clicks sidebar during detect.
**How to avoid:** Return cleanup function from useEffect or use useRef for interval ID, clear on unmount.
**Warning signs:** Console warnings about state updates on unmounted components.

### Pitfall 3: Case Sensitivity on Exe Names
**What goes wrong:** "notepad.exe" vs "Notepad.exe" treated as different apps.
**Why it happens:** Windows paths can have mixed case.
**How to avoid:** Backend already lowercases in `set_app_rule` (line 1154: `exe_name.to_lowercase()`). Frontend should also display lowercased names and compare lowercased.

### Pitfall 4: Reading Global ALL CAPS State for Inherit Label
**What goes wrong:** Inherit dropdown option needs to show "(ON)" or "(OFF)" but AllCapsToggle reads from the store, not from a shared state.
**Why it happens:** Global all_caps is read differently in AllCapsToggle (store.get) vs pipeline (ActiveProfile state).
**How to avoid:** Read global all_caps via `store.get<boolean>('all_caps')` or `invoke('get_all_caps')` on mount. The store approach is faster and doesn't require ActiveProfile state to be ready.

### Pitfall 5: Duplicate App Detection
**What goes wrong:** User detects the same app twice, creating confusion.
**Why it happens:** No dedup check before adding.
**How to avoid:** After detect, check if exe_name already exists in rules. If so, either show "already configured" message on button or scroll/highlight existing entry. Decision is Claude's discretion.

### Pitfall 6: VoiceType Itself as Detected App
**What goes wrong:** User clicks detect, but VoiceType's own window is still focused during the countdown.
**Why it happens:** 3-second countdown starts, user switches apps, but if they don't switch fast enough, VoiceType's own exe is detected.
**How to avoid:** The 3-second countdown is specifically designed for this -- user has time to switch. After detection, if the exe is the VoiceType binary itself, could show a helpful message. However, the countdown design inherently handles this.

## Code Examples

### Backend API Contract (verified from lib.rs)

```typescript
// Types matching Rust structs
interface DetectedApp {
  exe_name: string | null;
  window_title: string | null;
}

interface AppRule {
  all_caps: boolean | null;  // null = inherit, true = force ON, false = force OFF
}

// Commands (all registered and functional)
invoke<Record<string, AppRule>>('get_app_rules')
invoke('set_app_rule', { exeName: string, rule: AppRule })
invoke('remove_app_rule', { exeName: string })
invoke<DetectedApp>('detect_foreground_app')
invoke<boolean>('get_all_caps')  // or store.get<boolean>('all_caps')
```

### Sidebar ITEMS Array Pattern (from Sidebar.tsx)
```typescript
// Current order (line 15-22):
{ id: 'general', label: 'General', icon: Settings },
{ id: 'dictionary', label: 'Dictionary', icon: BookOpen },
// INSERT HERE: { id: 'app-rules', label: 'App Rules', icon: AppWindow },
{ id: 'model', label: 'Model', icon: Cpu },
{ id: 'appearance', label: 'Appearance', icon: Palette },
{ id: 'system', label: 'System', icon: Monitor },
{ id: 'history', label: 'History', icon: Clock },
```

### Card Pattern (from GeneralSection.tsx)
```typescript
<div className="bg-white dark:bg-gray-900 ring-1 ring-gray-200 dark:ring-gray-800 rounded-2xl p-4 shadow-sm">
  {/* Content */}
</div>
```

### Recommended Lucide Icons for Sidebar
Options from lucide-react that convey "per-app rules":
- `AppWindow` -- window with app grid, directly conveys "application"
- `SquareStack` -- stacked squares, suggests multiple apps
- `Layers` -- layers/overrides concept
- `SlidersHorizontal` -- settings/rules concept

Recommendation: `AppWindow` -- most semantically clear for "application-specific rules."

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Global-only ALL CAPS | Per-app overrides via Option<bool> | Phase 23-24 (this milestone) | Backend infrastructure ready, UI is the remaining piece |
| tauri-plugin-store | Custom SettingsState Mutex facade | Earlier in project | All settings go through invoke('get_setting')/invoke('set_setting') or dedicated commands |

## Open Questions

1. **Window title persistence for display**
   - What we know: AppRule only stores all_caps. Window title is available at detection time.
   - What's unclear: Should we persist window titles somewhere for display, or accept that rules loaded from settings won't show window titles?
   - Recommendation: Accept no window title on reload. The exe name is the primary identifier. Could optionally re-detect on hover or show "Last seen: ..." but that's over-engineering for v1.

2. **Invoke parameter naming: camelCase vs snake_case**
   - What we know: Tauri's `#[tauri::command]` macro auto-converts snake_case Rust params to camelCase for JS. Backend has `exe_name` in Rust.
   - What's unclear: Need to verify the exact JS-side param name.
   - Recommendation: Use `exeName` on the JS side (Tauri's default serde rename). Verified by looking at existing patterns like `set_all_caps` taking `{ enabled }`.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust cargo test (backend); no frontend test framework installed |
| Config file | Cargo.toml for Rust tests; none for frontend |
| Quick run command | `cd src-tauri && cargo test -- --test-threads=1` |
| Full suite command | `cd src-tauri && cargo test -- --test-threads=1` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| UI-01 | Sidebar page accessible | manual-only | N/A -- requires Tauri webview | N/A |
| UI-02 | View configured rules | manual-only | N/A -- frontend rendering | N/A |
| UI-03 | Detect Active App with countdown | manual-only | N/A -- requires Win32 + webview | N/A |
| UI-05 | Remove app from list | manual-only | N/A -- frontend + backend integration | N/A |
| OVR-01 | Three-state toggle | manual-only | N/A -- frontend UX | N/A |

**Justification for manual-only:** This phase is purely frontend UI work. The project has no frontend test framework (no vitest, jest, or testing-library). Backend commands are already unit-tested in foreground.rs (lines 186-331 -- extensive tests for AppRule serde, resolve_all_caps with all state combinations). The frontend work is layout, state management, and user interaction that requires a running Tauri webview to validate.

### Sampling Rate
- **Per task commit:** `cd src-tauri && cargo test -- --test-threads=1` (verify backend still passes)
- **Per wave merge:** Same + manual UI walkthrough
- **Phase gate:** Manual verification of all 4 success criteria

### Wave 0 Gaps
None -- existing backend test infrastructure covers the data layer. Frontend testing infrastructure is out of scope for this phase (would require adding vitest + testing-library, which is a separate concern).

## Sources

### Primary (HIGH confidence)
- `src-tauri/src/foreground.rs` -- AppRule struct, DetectedApp struct, resolve_all_caps function
- `src-tauri/src/lib.rs:1143-1190` -- Backend commands: get_app_rules, set_app_rule, remove_app_rule, detect_foreground_app
- `src-tauri/src/lib.rs:1813-1871` -- Command registration in invoke_handler
- `src/components/Sidebar.tsx` -- SectionId type, ITEMS array, sidebar rendering pattern
- `src/App.tsx:187-224` -- AnimatePresence page transition, section conditional rendering
- `src/components/sections/GeneralSection.tsx` -- Card pattern, header pattern, toggle layout
- `src/components/sections/DictionarySection.tsx` -- Section with invoke() data loading pattern
- `src/components/AllCapsToggle.tsx` -- Store reading pattern for global all_caps state
- `src/lib/store.ts` -- Settings store facade

### Secondary (MEDIUM confidence)
- lucide-react icon names -- based on library knowledge, AppWindow exists in lucide

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- no new libraries, using only what's in package.json
- Architecture: HIGH -- patterns directly observed from 6 existing section components
- Pitfalls: HIGH -- derived from reading actual backend code and data structures
- Backend API: HIGH -- commands verified in source code with exact signatures

**Research date:** 2026-03-07
**Valid until:** 2026-04-07 (stable -- no external dependencies to change)
