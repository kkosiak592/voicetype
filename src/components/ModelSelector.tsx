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
  onFp32Download?: () => void;
  fp32Downloading?: boolean;
  fp32Percent?: number;
  fp32Error?: string | null;
  onMoonshineDownload?: () => void;
  moonshineDownloading?: boolean;
  moonshinePercent?: number;
  moonshineError?: string | null;
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
  onFp32Download,
  fp32Downloading = false,
  fp32Percent = 0,
  fp32Error = null,
  onMoonshineDownload,
  moonshineDownloading = false,
  moonshinePercent = 0,
  moonshineError = null,
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
        const isParakeet = model.id === 'parakeet-tdt-v2-fp32';
        const isMoonshine = model.id === 'moonshine-tiny';

        // Resolve download state for Parakeet and Moonshine (both use dedicated download commands)
        const thisDownloading = isParakeet ? fp32Downloading : isMoonshine ? moonshineDownloading : false;
        const thisPercent = isParakeet ? fp32Percent : isMoonshine ? moonshinePercent : 0;
        const thisError = isParakeet ? fp32Error : isMoonshine ? moonshineError : null;
        const thisOnDownload = isParakeet ? onFp32Download : isMoonshine ? onMoonshineDownload : undefined;

        const isParakeetDownloading = isParakeet && thisDownloading;
        const isMoonshineDownloading = isMoonshine && thisDownloading;
        const isDownloading = isParakeet ? isParakeetDownloading
          : isMoonshine ? isMoonshineDownloading
            : downloadingId === model.id;
        const hasWhisperError =
          !isParakeet &&
          !isMoonshine &&
          downloadingId === null &&
          downloadError !== null &&
          !model.downloaded;
        const hasParakeetError = isParakeet && !model.downloaded && thisError !== null;
        const hasMoonshineError = isMoonshine && !model.downloaded && thisError !== null;
        const disabled = !model.downloaded || loadingId !== null || downloadingId !== null || fp32Downloading || moonshineDownloading;

        return (
          <div key={model.id}>
            <div
              onClick={() => model.downloaded && !disabled ? handleSelect(model) : undefined}
              role={model.downloaded ? 'button' : undefined}
              tabIndex={model.downloaded && !disabled ? 0 : undefined}
              onKeyDown={model.downloaded && !disabled ? (e) => { if (e.key === 'Enter' || e.key === ' ') handleSelect(model); } : undefined}
              className={[
                'w-full rounded-xl px-4 py-3 text-left transition-all duration-200',
                !model.downloaded
                  ? 'cursor-default bg-gray-50/50 dark:bg-gray-800/30 border border-dashed border-gray-300 dark:border-gray-600'
                  : isSelected
                    ? 'ring-2 ring-emerald-500 bg-emerald-50 dark:bg-emerald-500/10 dark:ring-emerald-500/80 shadow-sm cursor-pointer focus:outline-none'
                    : disabled
                      ? 'ring-1 ring-gray-200 bg-white opacity-50 dark:ring-gray-700 dark:bg-gray-800'
                      : 'ring-1 ring-gray-200 bg-white hover:ring-gray-300 hover:bg-gray-50 shadow-sm cursor-pointer dark:ring-gray-700 dark:bg-gray-800 hover:dark:ring-gray-600 focus:outline-none',
              ].join(' ')}
            >
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <span
                    className={[
                      'text-sm font-semibold',
                      isSelected && model.downloaded
                        ? 'text-emerald-700 dark:text-emerald-300'
                        : 'text-gray-900 dark:text-gray-100',
                    ].join(' ')}
                  >
                    {model.name}
                  </span>
                  {model.recommended && (
                    <span className="rounded-full bg-emerald-100/80 px-2.5 py-0.5 text-[10px] font-bold uppercase tracking-wider text-emerald-700 dark:bg-emerald-900/60 dark:text-emerald-300 border border-emerald-200 dark:border-emerald-800/50">
                      Recommended
                    </span>
                  )}
                </div>
                <div className="flex items-center gap-3">
                  {/* Parakeet download button */}
                  {isParakeet && !model.downloaded && thisOnDownload && !isParakeetDownloading && (
                    <button
                      onClick={(e) => { e.stopPropagation(); thisOnDownload(); }}
                      disabled={fp32Downloading || downloadingId !== null || moonshineDownloading}
                      className="rounded-lg bg-emerald-600 px-3 py-1.5 text-xs font-semibold tracking-wide text-white hover:bg-emerald-500 active:bg-emerald-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors shadow-sm"
                    >
                      Download
                    </button>
                  )}
                  {/* Moonshine download button */}
                  {isMoonshine && !model.downloaded && thisOnDownload && !isMoonshineDownloading && (
                    <button
                      onClick={(e) => { e.stopPropagation(); thisOnDownload(); }}
                      disabled={moonshineDownloading || fp32Downloading || downloadingId !== null}
                      className="rounded-lg bg-emerald-600 px-3 py-1.5 text-xs font-semibold tracking-wide text-white hover:bg-emerald-500 active:bg-emerald-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors shadow-sm"
                    >
                      Download
                    </button>
                  )}
                  {/* Whisper download button */}
                  {!isParakeet && !isMoonshine && !model.downloaded && !isDownloading && (
                    <button
                      onClick={(e) => { e.stopPropagation(); handleDownload(model.id); }}
                      disabled={downloadingId !== null || fp32Downloading || moonshineDownloading}
                      className="rounded-lg bg-emerald-600 px-3 py-1.5 text-xs font-semibold tracking-wide text-white hover:bg-emerald-500 active:bg-emerald-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors shadow-sm"
                    >
                      Download
                    </button>
                  )}
                  {isLoading && (
                    <span className="text-xs font-medium tracking-wide text-emerald-500 dark:text-emerald-400">Loading...</span>
                  )}
                </div>
              </div>
              <p className="mt-1 text-xs leading-relaxed text-gray-500 dark:text-gray-400 max-w-[90%]">{model.description}</p>
            </div>

            {/* Progress bar for Parakeet download */}
            {isParakeet && isParakeetDownloading && (
              <div className="mt-1 rounded-lg border border-gray-200 bg-gray-50 px-4 py-2.5 dark:border-gray-700 dark:bg-gray-800/50">
                <div className="h-1.5 w-full rounded-full bg-gray-200 dark:bg-gray-700 overflow-hidden">
                  <div
                    className="h-full rounded-full bg-emerald-500 transition-all duration-200"
                    style={{ width: `${thisPercent}%` }}
                  />
                </div>
                <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
                  {thisPercent}%
                </p>
              </div>
            )}

            {/* Error message for Parakeet download */}
            {hasParakeetError && thisError && (
              <div className="mt-1 flex items-center justify-between rounded-lg border border-red-200 bg-red-50 px-4 py-2 dark:border-red-800 dark:bg-red-900/20">
                <p className="text-xs text-red-600 dark:text-red-400 truncate">{thisError}</p>
                {thisOnDownload && (
                  <button
                    onClick={thisOnDownload}
                    className="ml-3 shrink-0 text-xs font-medium text-red-600 hover:text-red-700 dark:text-red-400 dark:hover:text-red-300"
                  >
                    Retry
                  </button>
                )}
              </div>
            )}

            {/* Progress bar for Moonshine download */}
            {isMoonshine && isMoonshineDownloading && (
              <div className="mt-1 rounded-lg border border-gray-200 bg-gray-50 px-4 py-2.5 dark:border-gray-700 dark:bg-gray-800/50">
                <div className="h-1.5 w-full rounded-full bg-gray-200 dark:bg-gray-700 overflow-hidden">
                  <div
                    className="h-full rounded-full bg-emerald-500 transition-all duration-200"
                    style={{ width: `${thisPercent}%` }}
                  />
                </div>
                <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
                  {thisPercent}%
                </p>
              </div>
            )}

            {/* Error message for Moonshine download */}
            {hasMoonshineError && thisError && (
              <div className="mt-1 flex items-center justify-between rounded-lg border border-red-200 bg-red-50 px-4 py-2 dark:border-red-800 dark:bg-red-900/20">
                <p className="text-xs text-red-600 dark:text-red-400 truncate">{thisError}</p>
                {thisOnDownload && (
                  <button
                    onClick={thisOnDownload}
                    className="ml-3 shrink-0 text-xs font-medium text-red-600 hover:text-red-700 dark:text-red-400 dark:hover:text-red-300"
                  >
                    Retry
                  </button>
                )}
              </div>
            )}

            {/* Progress bar for active Whisper download */}
            {!isParakeet && !isMoonshine && isDownloading && (
              <div className="mt-1 rounded-lg border border-gray-200 bg-gray-50 px-4 py-2.5 dark:border-gray-700 dark:bg-gray-800/50">
                <div className="h-1.5 w-full rounded-full bg-gray-200 dark:bg-gray-700 overflow-hidden">
                  <div
                    className="h-full rounded-full bg-emerald-500 transition-all duration-200"
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

            {/* Error message for Whisper download */}
            {!isMoonshine && hasWhisperError && downloadError && (
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
