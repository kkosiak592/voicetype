import { useState } from 'react';
import { invoke, Channel } from '@tauri-apps/api/core';

export interface ModelInfo {
  id: string;
  name: string;
  description: string;
  recommended: boolean;
  downloaded: boolean;
}

type DownloadEvent =
  | { event: 'started'; data: { url: string; totalBytes: number } }
  | { event: 'progress'; data: { downloadedBytes: number; totalBytes: number } }
  | { event: 'finished' }
  | { event: 'error'; data: { message: string } };

interface ModelSelectorProps {
  models: ModelInfo[];
  selectedId: string;
  onSelect: (id: string) => void;
  loading: boolean;
  onDownloadComplete?: (modelId: string) => void;
}

function formatMB(bytes: number): string {
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
}

export function ModelSelector({
  models,
  selectedId,
  onSelect,
  loading,
  onDownloadComplete,
}: ModelSelectorProps) {
  const [loadingId, setLoadingId] = useState<string | null>(null);
  const [downloadingId, setDownloadingId] = useState<string | null>(null);
  const [downloadedBytes, setDownloadedBytes] = useState(0);
  const [totalBytes, setTotalBytes] = useState(0);
  const [downloadError, setDownloadError] = useState<string | null>(null);

  async function handleSelect(model: ModelInfo) {
    if (!model.downloaded || model.id === selectedId || loadingId !== null) return;
    setLoadingId(model.id);
    await onSelect(model.id);
    setLoadingId(null);
  }

  async function handleDownload(modelId: string) {
    setDownloadingId(modelId);
    setDownloadedBytes(0);
    setTotalBytes(0);
    setDownloadError(null);

    const onEvent = new Channel<DownloadEvent>();
    onEvent.onmessage = (msg) => {
      switch (msg.event) {
        case 'started':
          setTotalBytes(msg.data.totalBytes);
          break;
        case 'progress':
          setDownloadedBytes(msg.data.downloadedBytes);
          setTotalBytes(msg.data.totalBytes);
          break;
        case 'finished':
          setDownloadingId(null);
          onDownloadComplete?.(modelId);
          break;
        case 'error':
          setDownloadError(msg.data.message);
          setDownloadingId(null);
          break;
      }
    };

    try {
      await invoke('download_model', { modelId, onEvent });
    } catch (e) {
      setDownloadError(String(e));
      setDownloadingId(null);
    }
  }

  if (loading) {
    return (
      <div className="space-y-2">
        {[0, 1].map((i) => (
          <div key={i} className="h-16 animate-pulse rounded-lg bg-gray-100 dark:bg-gray-800" />
        ))}
      </div>
    );
  }

  const percent =
    totalBytes > 0 ? Math.round((downloadedBytes / totalBytes) * 100) : null;

  return (
    <div className="space-y-2">
      {models.map((model) => {
        const isSelected = selectedId === model.id;
        const isLoading = loadingId === model.id;
        const isDownloading = downloadingId === model.id;
        const hasError = downloadingId === null && downloadError !== null && !model.downloaded;
        const disabled = !model.downloaded || loadingId !== null || downloadingId !== null;

        return (
          <div key={model.id}>
            <div
              onClick={() => model.downloaded && !disabled ? handleSelect(model) : undefined}
              role={model.downloaded ? 'button' : undefined}
              tabIndex={model.downloaded && !disabled ? 0 : undefined}
              onKeyDown={model.downloaded && !disabled ? (e) => { if (e.key === 'Enter' || e.key === ' ') handleSelect(model); } : undefined}
              className={[
                'w-full rounded-lg border-2 px-4 py-3 text-left transition-colors duration-150',
                !model.downloaded
                  ? 'cursor-default border-gray-200 bg-white dark:border-gray-700 dark:bg-gray-800'
                  : isSelected
                    ? 'border-indigo-500 bg-indigo-50 dark:border-indigo-400 dark:bg-indigo-950 cursor-pointer focus:outline-none'
                    : disabled
                      ? 'border-gray-200 bg-white opacity-50 dark:border-gray-700 dark:bg-gray-800'
                      : 'border-gray-200 bg-white hover:border-gray-300 cursor-pointer dark:border-gray-700 dark:bg-gray-800 dark:hover:border-gray-600 focus:outline-none',
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
                  {!model.downloaded && !isDownloading && (
                    <button
                      onClick={() => handleDownload(model.id)}
                      disabled={downloadingId !== null}
                      className="rounded-md bg-indigo-500 px-2.5 py-1 text-xs font-medium text-white hover:bg-indigo-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                    >
                      Download
                    </button>
                  )}
                  {isLoading && (
                    <span className="text-xs text-indigo-500 dark:text-indigo-400">Loading...</span>
                  )}
                </div>
              </div>
              <p className="mt-0.5 text-xs text-gray-500 dark:text-gray-400">{model.description}</p>
            </div>

            {/* Progress bar for active download */}
            {isDownloading && (
              <div className="mt-1 rounded-lg border border-gray-200 bg-gray-50 px-4 py-2.5 dark:border-gray-700 dark:bg-gray-800/50">
                <div className="h-1.5 w-full rounded-full bg-gray-200 dark:bg-gray-700 overflow-hidden">
                  <div
                    className="h-full rounded-full bg-indigo-500 transition-all duration-200"
                    style={{ width: percent !== null ? `${percent}%` : '40%' }}
                  />
                </div>
                <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
                  {percent !== null
                    ? `${percent}% — ${formatMB(downloadedBytes)} / ${formatMB(totalBytes)}`
                    : `Downloading... ${formatMB(downloadedBytes)}`}
                </p>
              </div>
            )}

            {/* Error message */}
            {hasError && downloadingId === null && downloadError && (
              <div className="mt-1 flex items-center justify-between rounded-lg border border-red-200 bg-red-50 px-4 py-2 dark:border-red-800 dark:bg-red-900/20">
                <p className="text-xs text-red-600 dark:text-red-400 truncate">{downloadError}</p>
                <button
                  onClick={() => handleDownload(model.id)}
                  className="ml-3 shrink-0 text-xs font-medium text-red-600 hover:text-red-700 dark:text-red-400 dark:hover:text-red-300"
                >
                  Retry
                </button>
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
