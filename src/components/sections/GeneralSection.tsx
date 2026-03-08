import { HotkeyCapture } from '../HotkeyCapture';
import { RecordingModeToggle } from '../RecordingModeToggle';
import { AllCapsToggle } from '../AllCapsToggle';
import { AlwaysListenToggle } from '../AlwaysListenToggle';
import { FillerRemovalToggle } from '../FillerRemovalToggle';
import { PrefixTextInput } from '../PrefixTextInput';

interface GeneralSectionProps {
  hotkey: string;
  onHotkeyChange: (key: string) => void;
  recordingMode: 'hold' | 'toggle';
  onRecordingModeChange: (mode: 'hold' | 'toggle') => void;
  hookAvailable: boolean;
}

export function GeneralSection({
  hotkey,
  onHotkeyChange,
  recordingMode,
  onRecordingModeChange,
  hookAvailable,
}: GeneralSectionProps) {
  return (
    <div>
      <div className="mb-4">
        <h1 className="text-xl font-bold tracking-tight text-gray-900 dark:text-gray-100">
          General Settings
        </h1>
        <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
          Configure how and when VoiceType listens to your voice.
        </p>
      </div>

      <div className="space-y-4">
        {/* Card 1: Activation */}
        <div className="bg-white dark:bg-gray-900 ring-1 ring-gray-200 dark:ring-gray-800 rounded-2xl p-4 shadow-sm">
          <section>
            <h2 className="text-sm font-semibold text-gray-900 dark:text-gray-100">
              Activation Hotkey
            </h2>
            <p className="mb-4 mt-1 text-sm text-gray-500 dark:text-gray-400">
              Click the box below then press your desired key combination to trigger recording.
            </p>
            <HotkeyCapture value={hotkey} onChange={onHotkeyChange} />
            {!hookAvailable && hotkey.split('+').every(k => ['ctrl', 'alt', 'shift', 'meta', 'win', 'super'].includes(k)) && (
              <p className="mt-3 text-xs text-amber-600 dark:text-amber-400 flex items-center gap-1.5">
                <span className="size-1.5 rounded-full bg-amber-500"></span>
                Hook unavailable — using standard shortcut fallback
              </p>
            )}
          </section>

          <div className="my-5 border-t border-gray-100 dark:border-gray-800" />

          <section>
            <h2 className="text-sm font-semibold text-gray-900 dark:text-gray-100">
              Recording Mode
            </h2>
            <p className="mb-4 mt-1 text-sm text-gray-500 dark:text-gray-400">
              Choose how the hotkey controls your recording session.
            </p>
            <RecordingModeToggle value={recordingMode} onChange={onRecordingModeChange} />
          </section>

          <div className="my-5 border-t border-gray-100 dark:border-gray-800" />

          <section>
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-medium text-gray-900 dark:text-gray-100">Always Listen</p>
                <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                  Keep microphone open to eliminate activation delay. Uses more resources.
                </p>
              </div>
              <AlwaysListenToggle />
            </div>
          </section>
        </div>
        {/* Card 2: Output */}
        <div className="bg-white dark:bg-gray-900 ring-1 ring-gray-200 dark:ring-gray-800 rounded-2xl p-4 shadow-sm">
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

            <div className="my-4 border-t border-gray-100 dark:border-gray-800" />

            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-medium text-gray-900 dark:text-gray-100">Remove Fillers</p>
                <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                  Strip hesitation sounds (um, uh, hmm) from transcribed text
                </p>
              </div>
              <FillerRemovalToggle />
            </div>

            <div className="my-4 border-t border-gray-100 dark:border-gray-800" />

            <PrefixTextInput />
          </section>
        </div>
      </div>
    </div>
  );
}
