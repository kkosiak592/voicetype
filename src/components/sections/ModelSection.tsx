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
      const engine = isParakeetVariant ? 'parakeet' : 'whisper';

      if (isParakeetVariant) {
        // Always call set_engine for Parakeet — reloads the model on every switch.
        await invoke('set_engine', { engine: 'parakeet', parakeetModel: modelId });
        setCurrentEngine('parakeet');
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

  return (
    <div>
      <h1 className="mb-1 text-base font-semibold tracking-tight text-gray-900 dark:text-gray-100">
        Model
      </h1>
      <p className="mb-4 text-sm text-gray-500 dark:text-gray-400">
        Select the transcription model. Additional models can be downloaded here.
      </p>

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
      />

      {gpuInfo && (
        <div className="mt-4 rounded-lg border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50 px-4 py-3">
          <p className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider mb-2">
            Inference Status
          </p>
          <div className="space-y-1">
            <div className="flex justify-between text-sm">
              <span className="text-gray-500 dark:text-gray-400">GPU</span>
              <span className="text-gray-900 dark:text-gray-100 font-medium">{gpuInfo.gpuName}</span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-gray-500 dark:text-gray-400">Provider</span>
              <span className="text-gray-900 dark:text-gray-100 font-medium">{gpuInfo.executionProvider}</span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-gray-500 dark:text-gray-400">Engine</span>
              <span className="text-gray-900 dark:text-gray-100 font-medium capitalize">{gpuInfo.activeEngine}</span>
            </div>
          </div>
        </div>
      )}

      {currentEngine === 'parakeet' && (
        <p className="mt-3 text-xs text-gray-400 dark:text-gray-500">
          Parakeet doesn't support vocabulary prompting. Your corrections dictionary still applies.
        </p>
      )}
    </div>
  );
}
