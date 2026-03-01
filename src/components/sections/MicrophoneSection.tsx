import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getStore } from '../../lib/store';

interface MicrophoneSectionProps {
  selectedMic: string;
  onSelectedMicChange: (device: string) => void;
}

export function MicrophoneSection({ selectedMic, onSelectedMicChange }: MicrophoneSectionProps) {
  const [devices, setDevices] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function loadDevices() {
      const deviceList = await invoke<string[]>('list_input_devices');
      setDevices(deviceList);
      setLoading(false);
    }
    loadDevices();
  }, []);

  async function handleDeviceChange(deviceName: string) {
    onSelectedMicChange(deviceName);
    const store = await getStore();
    await store.set('selectedMic', deviceName);
    await invoke('set_microphone', { deviceName });
  }

  return (
    <div>
      <h1 className="mb-1 text-base font-semibold tracking-tight text-gray-900 dark:text-gray-100">
        Microphone
      </h1>
      <p className="mb-4 text-sm text-gray-500 dark:text-gray-400">
        Select the input device for audio capture.
      </p>

      {loading ? (
        <div className="h-9 w-full animate-pulse rounded-lg bg-gray-100 dark:bg-gray-800" />
      ) : (
        <div>
          <select
            value={selectedMic}
            onChange={(e) => handleDeviceChange(e.target.value)}
            className="w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm focus:outline-none focus:border-indigo-400 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100"
          >
            {devices.map((device) => (
              <option key={device} value={device}>
                {device}
              </option>
            ))}
          </select>
          <p className="mt-1.5 text-xs text-gray-400 dark:text-gray-500">
            Audio stream will restart with the selected device.
          </p>
        </div>
      )}
    </div>
  );
}
