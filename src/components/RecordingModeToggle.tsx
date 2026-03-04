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
    <div className="flex gap-4">
      {OPTIONS.map((option) => {
        const isSelected = value === option.value;
        return (
          <button
            key={option.value}
            onClick={() => handleSelect(option.value)}
            className={[
              'relative flex flex-1 flex-col rounded-xl px-4 py-3.5 text-left transition-all duration-200 focus:outline-none',
              isSelected
                ? 'bg-emerald-50 dark:bg-emerald-500/10 ring-2 ring-emerald-500 dark:ring-emerald-500/80 shadow-sm'
                : 'bg-gray-50 dark:bg-gray-800/50 ring-1 ring-gray-200 dark:ring-gray-700 hover:bg-gray-100 dark:hover:bg-gray-800 hover:ring-gray-300 dark:hover:ring-gray-600',
            ].join(' ')}
          >
            {/* Optional dot indicator for selected state */}
            {isSelected && (
              <div className="absolute top-4 right-4 size-2 rounded-full bg-emerald-500" />
            )}
            <span
              className={[
                'text-sm font-semibold',
                isSelected
                  ? 'text-emerald-700 dark:text-emerald-300'
                  : 'text-gray-900 dark:text-gray-100',
              ].join(' ')}
            >
              {option.label}
            </span>
            <span className={['mt-1 text-xs leading-relaxed', isSelected ? 'text-emerald-600/70 dark:text-emerald-300/70' : 'text-gray-500 dark:text-gray-400'].join(' ')}>
              {option.description}
            </span>
          </button>
        );
      })}
    </div>
  );
}
