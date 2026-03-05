import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getStore } from '../../lib/store';

interface GpuInfo {
  gpuName: string;
  executionProvider: string;
  activeModel: string;
  activeEngine: string;
}

interface SystemSectionProps {
  selectedMic: string;
  onSelectedMicChange: (device: string) => void;
}

export function SystemSection({ selectedMic, onSelectedMicChange }: SystemSectionProps) {
  const [gpuInfo, setGpuInfo] = useState<GpuInfo | null>(null);
  const [devices, setDevices] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    invoke<GpuInfo>('get_gpu_info').then(setGpuInfo).catch(console.error);
  }, []);

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
          System
        </h1>
        <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
          Hardware and runtime information.
        </p>
      </div>

      <div className="space-y-4">
        {gpuInfo && (
          <div className="bg-white dark:bg-gray-900 ring-1 ring-gray-200 dark:ring-gray-800 rounded-2xl p-4 shadow-sm flex flex-col">
            <h2 className="text-sm font-semibold text-gray-900 dark:text-gray-100 mb-3">
              Inference Status
            </h2>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
              <div className="bg-gray-50 dark:bg-gray-800/50 rounded-xl p-3 ring-1 ring-gray-200/50 dark:ring-gray-700/50">
                <p className="text-[10px] font-semibold text-gray-400 dark:text-gray-500 uppercase tracking-wider mb-1.5">GPU</p>
                <p className="text-sm font-semibold text-gray-900 dark:text-gray-100 truncate">{gpuInfo.gpuName}</p>
              </div>
              <div className="bg-gray-50 dark:bg-gray-800/50 rounded-xl p-3 ring-1 ring-gray-200/50 dark:ring-gray-700/50">
                <p className="text-[10px] font-semibold text-gray-400 dark:text-gray-500 uppercase tracking-wider mb-1.5">Provider</p>
                <p className="text-sm font-semibold text-gray-900 dark:text-gray-100 truncate">{gpuInfo.executionProvider}</p>
              </div>
              <div className="bg-gray-50 dark:bg-gray-800/50 rounded-xl p-3 ring-1 ring-gray-200/50 dark:ring-gray-700/50">
                <p className="text-[10px] font-semibold text-gray-400 dark:text-gray-500 uppercase tracking-wider mb-1.5">Engine</p>
                <p className="text-sm font-semibold text-gray-900 dark:text-gray-100 capitalize truncate">{gpuInfo.activeEngine}</p>
              </div>
            </div>
          </div>
        )}

        <div className="bg-white dark:bg-gray-900 ring-1 ring-gray-200 dark:ring-gray-800 rounded-2xl p-4 shadow-sm">
          <h2 className="text-sm font-semibold text-gray-900 dark:text-gray-100 mb-3">
            Input Device
          </h2>
          {loading ? (
            <div className="h-11 w-full animate-pulse rounded-xl bg-gray-100 dark:bg-gray-800" />
          ) : (
            <div>
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
    </div>
  );
}
