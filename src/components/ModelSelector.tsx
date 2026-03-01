import { useState } from 'react';

export interface ModelInfo {
  id: string;
  name: string;
  description: string;
  recommended: boolean;
  downloaded: boolean;
}

interface ModelSelectorProps {
  models: ModelInfo[];
  selectedId: string;
  onSelect: (id: string) => void;
  loading: boolean;
}

export function ModelSelector({ models, selectedId, onSelect, loading }: ModelSelectorProps) {
  const [loadingId, setLoadingId] = useState<string | null>(null);

  async function handleSelect(model: ModelInfo) {
    if (!model.downloaded || model.id === selectedId || loadingId !== null) return;
    setLoadingId(model.id);
    await onSelect(model.id);
    setLoadingId(null);
  }

  if (loading) {
    return (
      <div className="space-y-2">
        {[0, 1, 2].map((i) => (
          <div key={i} className="h-16 animate-pulse rounded-lg bg-gray-100 dark:bg-gray-800" />
        ))}
      </div>
    );
  }

  return (
    <div className="space-y-2">
      {models.map((model) => {
        const isSelected = selectedId === model.id;
        const isLoading = loadingId === model.id;
        const disabled = !model.downloaded || loadingId !== null;

        return (
          <button
            key={model.id}
            onClick={() => handleSelect(model)}
            disabled={disabled}
            className={[
              'w-full rounded-lg border-2 px-4 py-3 text-left transition-colors duration-150 focus:outline-none',
              !model.downloaded
                ? 'cursor-not-allowed border-gray-200 bg-white opacity-50 dark:border-gray-700 dark:bg-gray-800'
                : isSelected
                  ? 'border-indigo-500 bg-indigo-50 dark:border-indigo-400 dark:bg-indigo-950'
                  : 'border-gray-200 bg-white hover:border-gray-300 dark:border-gray-700 dark:bg-gray-800 dark:hover:border-gray-600',
            ].join(' ')}
          >
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <span
                  className={[
                    'text-sm font-medium',
                    isSelected && model.downloaded
                      ? 'text-indigo-700 dark:text-indigo-300'
                      : 'text-gray-900 dark:text-gray-100',
                  ].join(' ')}
                >
                  {model.name}
                </span>
                {model.recommended && (
                  <span className="rounded-full bg-indigo-100 px-2 py-0.5 text-xs font-medium text-indigo-700 dark:bg-indigo-900 dark:text-indigo-300">
                    Recommended
                  </span>
                )}
              </div>
              <div className="flex items-center gap-2">
                {!model.downloaded && (
                  <span className="text-xs text-gray-400 dark:text-gray-500">Not downloaded</span>
                )}
                {isLoading && (
                  <span className="text-xs text-indigo-500 dark:text-indigo-400">Loading...</span>
                )}
              </div>
            </div>
            <p className="mt-0.5 text-xs text-gray-500 dark:text-gray-400">{model.description}</p>
          </button>
        );
      })}
    </div>
  );
}
