import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ModelSelector, ModelInfo } from '../ModelSelector';
import { getStore } from '../../lib/store';

interface ModelSectionProps {
  selectedModel: string;
  onSelectedModelChange: (id: string) => void;
}

export function ModelSection({ selectedModel, onSelectedModelChange }: ModelSectionProps) {
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadModels().catch(err => {
      console.error('Failed to load models:', err);
      setLoading(false);
    });
  }, []);

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

  return (
    <div>
      <h1 className="mb-1 text-base font-semibold tracking-tight text-gray-900 dark:text-gray-100">
        Model
      </h1>
      <p className="mb-4 text-sm text-gray-500 dark:text-gray-400">
        Select the whisper model for transcription. Additional models can be downloaded here.
      </p>

      <ModelSelector
        models={models}
        selectedId={selectedModel}
        onSelect={handleModelSelect}
        loading={loading}
        onDownloadComplete={handleDownloadComplete}
      />
    </div>
  );
}
