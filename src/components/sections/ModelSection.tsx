import { useEffect, useState } from 'react';
import { invoke, Channel } from '@tauri-apps/api/core';
import { ModelSelector, ModelInfo } from '../ModelSelector';
import { getStore } from '../../lib/store';

type DownloadEvent =
  | { event: 'started'; data: { url: string; totalBytes: number } }
  | { event: 'progress'; data: { downloadedBytes: number; totalBytes: number } }
  | { event: 'finished' }
  | { event: 'error'; data: { message: string } };

interface GpuInfo {
  gpuName: string;
  executionProvider: string;
  activeModel: string;
  activeEngine: string;
}

interface ModelSectionProps {
  selectedModel: string;
  onSelectedModelChange: (id: string) => void;
}

export function ModelSection({ selectedModel, onSelectedModelChange }: ModelSectionProps) {
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [currentEngine, setCurrentEngine] = useState<string>('whisper');
  const [fp32Downloading, setFp32Downloading] = useState(false);
  const [fp32Percent, setFp32Percent] = useState(0);
  const [fp32Error, setFp32Error] = useState<string | null>(null);
  const [moonshineDownloading, setMoonshineDownloading] = useState(false);
  const [moonshinePercent, setMoonshinePercent] = useState(0);
  const [moonshineError, setMoonshineError] = useState<string | null>(null);
  const [gpuInfo, setGpuInfo] = useState<GpuInfo | null>(null);

  useEffect(() => {
    loadModels().catch(err => {
      console.error('Failed to load models:', err);
      setLoading(false);
    });
    loadEngine().catch(err => {
      console.error('Failed to load engine:', err);
    });
  }, []);

  useEffect(() => {
    invoke<GpuInfo>('get_gpu_info').then(setGpuInfo).catch(console.error);
  }, [selectedModel, currentEngine]);

  async function loadEngine() {
    try {
      const engine = await invoke<string>('get_engine');
      setCurrentEngine(engine);
    } catch (err) {
      console.error('Failed to get engine:', err);
    }
  }

  async function loadModels() {
    const modelList = await invoke<ModelInfo[]>('list_models');
    setModels(modelList);
    setLoading(false);
  }

  async function handleModelSelect(modelId: string) {
    try {
      const isParakeetVariant = modelId === 'parakeet-tdt-v2-fp32';
      const isMoonshineVariant = modelId === 'moonshine-tiny';
      const engine = isParakeetVariant ? 'parakeet'
        : isMoonshineVariant ? 'moonshine'
          : 'whisper';

      if (isParakeetVariant) {
        // Always call set_engine for Parakeet — reloads the model on every switch.
        await invoke('set_engine', { engine: 'parakeet', parakeetModel: modelId });
        setCurrentEngine('parakeet');
      } else if (isMoonshineVariant) {
        await invoke('set_engine', { engine: 'moonshine', parakeetModel: null });
        setCurrentEngine('moonshine');
      } else {
        if (engine !== currentEngine) {
          await invoke('set_engine', { engine, parakeetModel: null });
          setCurrentEngine(engine);
        }
        await invoke('set_model', { modelId });
      }

      const store = await getStore();
      await store.set('selectedModel', modelId);
      onSelectedModelChange(modelId);
    } catch (err) {
      console.error('Failed to set model:', err);
    }
  }

  async function handleDownloadComplete(modelId: string) {
    const modelList = await invoke<ModelInfo[]>('list_models');
    setModels(modelList);
    // Auto-select the freshly downloaded model with engine-aware logic
    await handleModelSelect(modelId);
  }

  async function handleFp32Download() {
    setFp32Downloading(true);
    setFp32Percent(0);
    setFp32Error(null);

    const onEvent = new Channel<DownloadEvent>();
    onEvent.onmessage = async (msg) => {
      switch (msg.event) {
        case 'started':
          break;
        case 'progress': {
          const pct =
            msg.data.totalBytes > 0
              ? Math.round((msg.data.downloadedBytes / msg.data.totalBytes) * 100)
              : 0;
          setFp32Percent(pct);
          break;
        }
        case 'finished':
          setFp32Downloading(false);
          await loadModels();
          await handleModelSelect('parakeet-tdt-v2-fp32');
          break;
        case 'error':
          setFp32Error(msg.data.message);
          setFp32Downloading(false);
          break;
      }
    };

    try {
      await invoke('download_parakeet_fp32_model', { onEvent });
    } catch (e) {
      setFp32Error(String(e));
      setFp32Downloading(false);
    }
  }

  async function handleMoonshineDownload() {
    setMoonshineDownloading(true);
    setMoonshinePercent(0);
    setMoonshineError(null);

    const onEvent = new Channel<DownloadEvent>();
    onEvent.onmessage = async (msg) => {
      switch (msg.event) {
        case 'started':
          break;
        case 'progress': {
          const pct = msg.data.totalBytes > 0
            ? Math.round((msg.data.downloadedBytes / msg.data.totalBytes) * 100)
            : 0;
          setMoonshinePercent(pct);
          break;
        }
        case 'finished':
          setMoonshineDownloading(false);
          await loadModels();
          await handleModelSelect('moonshine-tiny');
          break;
        case 'error':
          setMoonshineError(msg.data.message);
          setMoonshineDownloading(false);
          break;
      }
    };

    try {
      await invoke('download_moonshine_tiny_model', { onEvent });
    } catch (e) {
      setMoonshineError(String(e));
      setMoonshineDownloading(false);
    }
  }

  return (
    <div>
      <div className="mb-4">
        <h1 className="text-xl font-bold tracking-tight text-gray-900 dark:text-gray-100">
          Model
        </h1>
        <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
          Select the transcription model. Additional models can be downloaded here.
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
          <ModelSelector
            models={models}
            selectedId={selectedModel}
            onSelect={handleModelSelect}
            loading={loading}
            onDownloadComplete={handleDownloadComplete}
            onFp32Download={handleFp32Download}
            fp32Downloading={fp32Downloading}
            fp32Percent={fp32Percent}
            fp32Error={fp32Error}
            onMoonshineDownload={handleMoonshineDownload}
            moonshineDownloading={moonshineDownloading}
            moonshinePercent={moonshinePercent}
            moonshineError={moonshineError}
          />
        </div>



        {(currentEngine === 'parakeet' || currentEngine === 'moonshine') && (
          <p className="text-xs text-amber-600 dark:text-amber-400 flex items-center gap-1.5 px-2">
            <span className="size-1.5 rounded-full bg-amber-500"></span>
            {currentEngine.charAt(0).toUpperCase() + currentEngine.slice(1)} doesn't support vocabulary prompting. Your corrections dictionary still applies.
          </p>
        )}
      </div>
    </div>
  );
}
