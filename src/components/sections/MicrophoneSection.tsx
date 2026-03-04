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
    loadDevices().catch(err => {
      console.error('Failed to load devices:', err);
      setLoading(false);
    });
  }, []);

  async function handleDeviceChange(deviceName: string) {
    try {
      await invoke('set_microphone', { deviceName });
      const store = await getStore();
      await store.set('selectedMic', deviceName);
      onSelectedMicChange(deviceName);
    } catch (err) {
      console.error('Failed to set microphone:', err);
    }
  }

  return (
    <div>
      <div className="mb-4">
        <h1 className="text-xl font-bold tracking-tight text-gray-900 dark:text-gray-100">
          Microphone
        </h1>
        <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
          Select the input device for audio capture.
        </p>
      </div>

      <div className="bg-white dark:bg-gray-900 ring-1 ring-gray-200 dark:ring-gray-800 rounded-2xl p-4 shadow-sm">
        {loading ? (
          <div className="h-11 w-full animate-pulse rounded-xl bg-gray-100 dark:bg-gray-800" />
        ) : (
          <div>
            <label htmlFor="mic-select" className="block text-sm font-semibold text-gray-900 dark:text-gray-100 mb-3">
              Input Device
            </label>
            <select
              id="mic-select"
              value={selectedMic}
              onChange={(e) => handleDeviceChange(e.target.value)}
              className="w-full rounded-xl border border-gray-300 bg-gray-50 px-4 py-2.5 text-sm font-medium text-gray-900 focus:outline-none focus:ring-2 focus:ring-emerald-500/50 focus:border-emerald-500 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-100 dark:focus:ring-emerald-500/50 shadow-inner"
            >
              {devices.map((device) => (
                <option key={device} value={device}>
                  {device}
                </option>
              ))}
            </select>
            <p className="mt-3 text-xs text-gray-500 dark:text-gray-400 flex items-center gap-1.5">
              <span className="size-1.5 rounded-full bg-emerald-500"></span>
              Audio stream will automatically restart when you change the device.
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
