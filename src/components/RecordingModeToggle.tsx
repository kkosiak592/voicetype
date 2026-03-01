import { invoke } from '@tauri-apps/api/core';
import { getStore } from '../lib/store';

interface RecordingModeToggleProps {
  value: 'hold' | 'toggle';
  onChange: (mode: 'hold' | 'toggle') => void;
}

const OPTIONS: Array<{
  value: 'hold' | 'toggle';
  label: string;
  description: string;
}> = [
  {
    value: 'hold',
    label: 'Hold to talk',
    description: 'Hold the hotkey while speaking. Release to transcribe.',
  },
  {
    value: 'toggle',
    label: 'Toggle',
    description: 'Tap to start. Tap again or wait for auto-stop.',
  },
];

export function RecordingModeToggle({ value, onChange }: RecordingModeToggleProps) {
  async function handleSelect(mode: 'hold' | 'toggle') {
    if (mode === value) return;

    // Update backend managed state immediately
    await invoke('set_recording_mode', { mode });

    // Persist to frontend store for UI consistency
    const store = await getStore();
    await store.set('recordingMode', mode);

    onChange(mode);
  }

  return (
    <div className="flex gap-3">
      {OPTIONS.map((option) => {
        const isSelected = value === option.value;
        return (
          <button
            key={option.value}
            onClick={() => handleSelect(option.value)}
            className={[
              'flex flex-1 flex-col rounded-lg border-2 px-3 py-2.5 text-left transition-colors duration-150 focus:outline-none',
              isSelected
                ? 'border-indigo-500 bg-indigo-50 dark:border-indigo-400 dark:bg-indigo-950'
                : 'border-gray-200 bg-white hover:border-gray-300 dark:border-gray-700 dark:bg-gray-800 dark:hover:border-gray-600',
            ].join(' ')}
          >
            <span
              className={[
                'text-sm font-medium',
                isSelected
                  ? 'text-indigo-700 dark:text-indigo-300'
                  : 'text-gray-900 dark:text-gray-100',
              ].join(' ')}
            >
              {option.label}
            </span>
            <span className="mt-0.5 text-xs text-gray-500 dark:text-gray-400">
              {option.description}
            </span>
          </button>
        );
      })}
    </div>
  );
}
