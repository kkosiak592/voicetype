import { useEffect, useState } from 'react';
import { getVersion } from '@tauri-apps/api/app';
import { HotkeyCapture } from '../HotkeyCapture';
import { RecordingModeToggle } from '../RecordingModeToggle';
import type { UpdateState } from '../../lib/updater';

interface GeneralSectionProps {
  hotkey: string;
  onHotkeyChange: (key: string) => void;
  recordingMode: 'hold' | 'toggle';
  onRecordingModeChange: (mode: 'hold' | 'toggle') => void;
  updaterState: UpdateState;
  onCheckForUpdates: () => Promise<void>;
  hookAvailable: boolean;
}

export function GeneralSection({
  hotkey,
  onHotkeyChange,
  recordingMode,
  onRecordingModeChange,
  updaterState,
  onCheckForUpdates,
  hookAvailable,
}: GeneralSectionProps) {
  const [appVersion, setAppVersion] = useState<string>('');

  useEffect(() => {
    getVersion().then(setAppVersion).catch(() => setAppVersion(''));
  }, []);

  const { status, lastChecked } = updaterState;
  const isChecking = status === 'checking';
  const hasUpdate = status === 'available' || status === 'ready';
  const isUpToDate = status === 'idle' && lastChecked > 0;

  function renderCheckButton() {
    if (isChecking) {
      return (
        <button
          disabled
          className="text-sm text-gray-400 dark:text-gray-500 cursor-not-allowed"
        >
          Checking...
        </button>
      );
    }

    if (hasUpdate) {
      return (
        <span className="text-sm font-medium text-green-600 dark:text-green-400">
          Update available
        </span>
      );
    }

    return (
      <button
        onClick={onCheckForUpdates}
        className="text-sm text-indigo-600 hover:underline focus:outline-none dark:text-indigo-400"
      >
        Check for Updates
      </button>
    );
  }

  return (
    <div>
      <h1 className="mb-5 text-base font-semibold tracking-tight text-gray-900 dark:text-gray-100">
        General
      </h1>

      <div className="space-y-5">
        {/* Hotkey subsection */}
        <section>
          <h2 className="mb-1 text-xs font-medium uppercase tracking-wider text-gray-500 dark:text-gray-400">
            Hotkey
          </h2>
          <p className="mb-2 text-sm text-gray-500 dark:text-gray-400">
            Click the box below then press your desired key combination.
          </p>
          <HotkeyCapture value={hotkey} onChange={onHotkeyChange} />
          {!hookAvailable && (
            <p className="mt-1 text-xs text-amber-600 dark:text-amber-400">
              Hook unavailable — using standard shortcut fallback
            </p>
          )}
        </section>

        <hr className="border-gray-200 dark:border-gray-700" />

        {/* Recording Mode subsection */}
        <section>
          <h2 className="mb-1 text-xs font-medium uppercase tracking-wider text-gray-500 dark:text-gray-400">
            Recording Mode
          </h2>
          <p className="mb-2 text-sm text-gray-500 dark:text-gray-400">
            Choose how the hotkey controls recording.
          </p>
          <RecordingModeToggle value={recordingMode} onChange={onRecordingModeChange} />
        </section>

        <hr className="border-gray-200 dark:border-gray-700" />

        {/* Updates subsection */}
        <section>
          <h2 className="mb-1 text-xs font-medium uppercase tracking-wider text-gray-500 dark:text-gray-400">
            Updates
          </h2>

          <div className="mt-2 flex items-center gap-3">
            {renderCheckButton()}
            {isUpToDate && !hasUpdate && !isChecking && (
              <span className="text-sm text-gray-400 dark:text-gray-500">Up to date</span>
            )}
          </div>

          {appVersion && (
            <p className="mt-4 text-xs text-gray-400 dark:text-gray-500">
              VoiceType v{appVersion}
            </p>
          )}
        </section>
      </div>
    </div>
  );
}
