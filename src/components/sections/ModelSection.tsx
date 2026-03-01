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
      // Determine the correct engine based on the selected model
      const engine = modelId === 'parakeet-tdt-v2' ? 'parakeet' : 'whisper';

      // Switch engine if needed
      if (engine !== currentEngine) {
        await invoke('set_engine', { engine });
        setCurrentEngine(engine);
      }

      // For Whisper models, also call set_model
      if (engine === 'whisper') {
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
      />

      {currentEngine === 'parakeet' && (
        <p className="mt-3 text-xs text-gray-400 dark:text-gray-500">
          Parakeet doesn't support vocabulary prompting. Your corrections dictionary still applies.
        </p>
      )}
    </div>
  );
}
