import { HotkeyCapture } from '../HotkeyCapture';
import { RecordingModeToggle } from '../RecordingModeToggle';

interface GeneralSectionProps {
  hotkey: string;
  onHotkeyChange: (key: string) => void;
  recordingMode: 'hold' | 'toggle';
  onRecordingModeChange: (mode: 'hold' | 'toggle') => void;
}

export function GeneralSection({
  hotkey,
  onHotkeyChange,
  recordingMode,
  onRecordingModeChange,
}: GeneralSectionProps) {
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
      </div>
    </div>
  );
}
