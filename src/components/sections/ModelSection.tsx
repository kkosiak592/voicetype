import { useEffect, useState } from 'react';
import { invoke, Channel } from '@tauri-apps/api/core';
import { ModelSelector, ModelInfo } from '../ModelSelector';
import { getStore } from '../../lib/store';

type DownloadEvent =
  | { event: 'started'; data: { url: string; totalBytes: number } }
  | { event: 'progress'; data: { downloadedBytes: number; totalBytes: number } }
  | { event: 'finished' }
  | { event: 'error'; data: { message: string } };

interface ModelSectionProps {
  selectedModel: string;
  onSelectedModelChange: (id: string) => void;
}

export function ModelSection({ selectedModel, onSelectedModelChange }: ModelSectionProps) {
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [currentEngine, setCurrentEngine] = useState<string>('whisper');
  const [parakeetDownloading, setParakeetDownloading] = useState(false);
  const [parakeetPercent, setParakeetPercent] = useState(0);
  const [parakeetError, setParakeetError] = useState<string | null>(null);
  const [fp32Downloading, setFp32Downloading] = useState(false);
  const [fp32Percent, setFp32Percent] = useState(0);
  const [fp32Error, setFp32Error] = useState<string | null>(null);

  useEffect(() => {
    loadModels().catch(err => {
      console.error('Failed to load models:', err);
      setLoading(false);
    });
    loadEngine().catch(err => {
      console.error('Failed to load engine:', err);
    });
  }, []);

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
      const isParakeetVariant = modelId === 'parakeet-tdt-v2' || modelId === 'parakeet-tdt-v2-fp32';
      const engine = isParakeetVariant ? 'parakeet' : 'whisper';

      if (isParakeetVariant) {
        // Always call set_engine for Parakeet variants regardless of currentEngine.
        // This is necessary because variant switches (int8 -> fp32 or fp32 -> int8)
        // require a model reload even when the engine is already "parakeet".
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

  async function handleParakeetDownload() {
    setParakeetDownloading(true);
    setParakeetPercent(0);
    setParakeetError(null);

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
          setParakeetPercent(pct);
          break;
        }
        case 'finished':
          setParakeetDownloading(false);
          // Refresh model list then auto-select Parakeet (implicitly sets engine)
          await loadModels();
          await handleModelSelect('parakeet-tdt-v2');
          break;
        case 'error':
          setParakeetError(msg.data.message);
          setParakeetDownloading(false);
          break;
      }
    };

    try {
      await invoke('download_parakeet_model', { onEvent });
    } catch (e) {
      setParakeetError(String(e));
      setParakeetDownloading(false);
    }
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
        onParakeetDownload={handleParakeetDownload}
        parakeetDownloading={parakeetDownloading}
        parakeetPercent={parakeetPercent}
        parakeetError={parakeetError}
        onFp32Download={handleFp32Download}
        fp32Downloading={fp32Downloading}
        fp32Percent={fp32Percent}
        fp32Error={fp32Error}
      />

      {currentEngine === 'parakeet' && (
        <p className="mt-3 text-xs text-gray-400 dark:text-gray-500">
          Parakeet doesn't support vocabulary prompting. Your corrections dictionary still applies.
        </p>
      )}
    </div>
  );
}
