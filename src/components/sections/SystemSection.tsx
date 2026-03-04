import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface GpuInfo {
  gpuName: string;
  executionProvider: string;
  activeModel: string;
  activeEngine: string;
}

export function SystemSection() {
  const [gpuInfo, setGpuInfo] = useState<GpuInfo | null>(null);

  useEffect(() => {
    invoke<GpuInfo>('get_gpu_info').then(setGpuInfo).catch(console.error);
  }, []);

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
      </div>
    </div>
  );
}
