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
      await invoke('set_model', { modelId });
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
    // Auto-select the freshly downloaded model
    await handleModelSelect(modelId);
  }

  async function handleEngineChange(engine: string) {
    try {
      await invoke('set_engine', { engine });
      setCurrentEngine(engine);
    } catch (err) {
      console.error('Failed to set engine:', err);
    }
  }

  async function handleParakeetDownload() {
    setParakeetDownloading(true);
    setParakeetPercent(0);
    setParakeetError(null);

    const onEvent = new Channel<DownloadEvent>();
    onEvent.onmessage = (msg) => {
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
          // Refresh model list so parakeetDownloaded becomes true
          loadModels().catch(console.error);
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

  const parakeetModel = models.find((m) => m.id === 'parakeet-tdt-v2');
  const parakeetDownloaded = parakeetModel?.downloaded ?? false;
  const whisperModels = models.filter((m) => m.id !== 'parakeet-tdt-v2');
  // GPU is present if any model with gpuOnly characteristics exists (large-v3-turbo or parakeet-tdt-v2)
  const hasGpu = models.some((m) => m.id === 'large-v3-turbo' || m.id === 'parakeet-tdt-v2');

  return (
    <div>
      <h1 className="mb-1 text-base font-semibold tracking-tight text-gray-900 dark:text-gray-100">
        Model
      </h1>
      <p className="mb-4 text-sm text-gray-500 dark:text-gray-400">
        Select the transcription model. Additional models can be downloaded here.
      </p>

      {/* Engine selector — GPU users only */}
      {hasGpu && (
        <div className="mb-6">
          <h2 className="mb-2 text-sm font-medium text-gray-700 dark:text-gray-300">
            Transcription Engine
          </h2>
          <div className="flex gap-2">
            <button
              onClick={() => handleEngineChange('whisper')}
              className={`rounded-lg border-2 px-4 py-2 text-sm font-medium transition-colors ${
                currentEngine === 'whisper'
                  ? 'border-indigo-500 bg-indigo-50 text-indigo-700 dark:border-indigo-400 dark:bg-indigo-950 dark:text-indigo-300'
                  : 'border-gray-200 bg-white text-gray-600 hover:border-gray-300 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-400'
              }`}
            >
              Whisper
              <span className="ml-1 text-xs text-gray-400">(Accurate)</span>
            </button>
            <button
              onClick={() => handleEngineChange('parakeet')}
              disabled={!parakeetDownloaded}
              className={`rounded-lg border-2 px-4 py-2 text-sm font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed ${
                currentEngine === 'parakeet'
                  ? 'border-indigo-500 bg-indigo-50 text-indigo-700 dark:border-indigo-400 dark:bg-indigo-950 dark:text-indigo-300'
                  : 'border-gray-200 bg-white text-gray-600 hover:border-gray-300 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-400'
              }`}
            >
              Parakeet TDT
              <span className="ml-1 text-xs text-gray-400">(Fast)</span>
            </button>
          </div>
          {currentEngine === 'parakeet' && (
            <p className="mt-2 text-xs text-gray-400 dark:text-gray-500">
              Parakeet doesn't support vocabulary prompting. Your corrections dictionary still applies.
            </p>
          )}
        </div>
      )}

      {/* Parakeet download prompt — GPU users who don't have it yet */}
      {hasGpu && !parakeetDownloaded && (
        <div className="mb-4 rounded-lg border border-dashed border-gray-300 dark:border-gray-600 p-3">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-700 dark:text-gray-300">Parakeet TDT</p>
              <p className="text-xs text-gray-400">661 MB — Download to enable fast engine</p>
            </div>
            <button
              onClick={handleParakeetDownload}
              disabled={parakeetDownloading}
              className="rounded-md bg-indigo-500 px-3 py-1.5 text-xs font-medium text-white hover:bg-indigo-600 disabled:opacity-50 transition-colors"
            >
              {parakeetDownloading ? `${parakeetPercent}%` : 'Download'}
            </button>
          </div>
          {parakeetDownloading && (
            <div className="mt-2 h-1.5 w-full rounded-full bg-gray-200 dark:bg-gray-700 overflow-hidden">
              <div
                className="h-full rounded-full bg-indigo-500 transition-all"
                style={{ width: `${parakeetPercent}%` }}
              />
            </div>
          )}
          {parakeetError && (
            <p className="mt-2 text-xs text-red-600 dark:text-red-400 break-all">{parakeetError}</p>
          )}
        </div>
      )}

      <ModelSelector
        models={whisperModels}
        selectedId={selectedModel}
        onSelect={handleModelSelect}
        loading={loading}
        onDownloadComplete={handleDownloadComplete}
      />
    </div>
  );
}
