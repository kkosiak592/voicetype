import { useEffect, useRef, useState } from 'react';
import { invoke, Channel } from '@tauri-apps/api/core';

type DownloadEvent =
  | { event: 'started'; data: { url: string; totalBytes: number } }
  | { event: 'progress'; data: { downloadedBytes: number; totalBytes: number } }
  | { event: 'finished' }
  | { event: 'error'; data: { message: string } };

type DownloadState = 'idle' | 'downloading' | 'validating' | 'complete' | 'error';

interface FirstRunProps {
  gpuDetected: boolean;
  gpuName: string;
  directmlAvailable: boolean;
  recommendedModel: string;
  onComplete: (downloadedModelId: string) => void;
}

const MODELS = [
  {
    id: 'large-v3-turbo',
    name: 'Large v3 Turbo',
    size: '574 MB',
    quality: 'Most accurate',
    requirement: 'NVIDIA GPU required',
    gpuOnly: true,
  },
  {
    id: 'distil-large-v3.5',
    name: 'Distil Large v3.5',
    size: '1.52 GB',
    quality: 'High accuracy, fast',
    requirement: 'GPU recommended, works on any hardware',
    gpuOnly: false,
  },
  {
    id: 'parakeet-tdt-v2-fp32',
    name: 'Parakeet TDT (fp32)',
    size: '2.56 GB',
    quality: 'Fast and accurate',
    requirement: 'GPU accelerated (CUDA or DirectML)',
    gpuOnly: false,
  },
  {
    id: 'small-en',
    name: 'Small (English)',
    size: '190 MB',
    quality: 'Fast and lightweight',
    requirement: 'GPU accelerated when available',
    gpuOnly: false,
  },
];

function formatMB(bytes: number): string {
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
}

export function FirstRun({ gpuDetected, gpuName, directmlAvailable, recommendedModel, onComplete }: FirstRunProps) {
  const [downloadState, setDownloadState] = useState<DownloadState>('idle');
  const [downloadingId, setDownloadingId] = useState<string | null>(null);
  const [downloadedBytes, setDownloadedBytes] = useState(0);
  const [totalBytes, setTotalBytes] = useState(0);
  const [errorMsg, setErrorMsg] = useState('');
  const cancelledRef = useRef(false);

  const visibleModels = MODELS.filter((m) => {
    if (m.id === 'parakeet-tdt-v2-fp32') {
      return gpuDetected || directmlAvailable;
    }
    if (m.gpuOnly) {
      return gpuDetected;
    }
    return true;
  });

  useEffect(() => {
    if (downloadState !== 'complete') return;

    let active = true;

    async function handleComplete() {
      try {
        await invoke('enable_autostart');
      } catch (e) {
        console.warn('Failed to enable autostart:', e);
        // Non-blocking — user can toggle autostart later in settings
      }

      // If the user chose Parakeet fp32, activate that engine immediately
      if (downloadingId === 'parakeet-tdt-v2-fp32') {
        try {
          await invoke('set_engine', { engine: 'parakeet', parakeetModel: downloadingId });
        } catch (e) {
          console.warn('Failed to set Parakeet engine:', e);
        }
      }

      if (active && downloadingId) {
        setTimeout(() => {
          if (active) onComplete(downloadingId);
        }, 1000);
      }
    }

    handleComplete();

    return () => {
      active = false;
    };
  }, [downloadState, onComplete]);

  async function handleDownload(modelId: string) {
    cancelledRef.current = false;
    setDownloadingId(modelId);
    setDownloadedBytes(0);
    setTotalBytes(0);
    setErrorMsg('');
    setDownloadState('downloading');

    const onEvent = new Channel<DownloadEvent>();
    onEvent.onmessage = (msg) => {
      if (cancelledRef.current) return;

      switch (msg.event) {
        case 'started':
          setTotalBytes(msg.data.totalBytes);
          setDownloadState('downloading');
          break;
        case 'progress':
          setDownloadedBytes(msg.data.downloadedBytes);
          setTotalBytes(msg.data.totalBytes);
          break;
        case 'finished':
          setDownloadState('complete');
          break;
        case 'error':
          setErrorMsg(msg.data.message);
          setDownloadState('error');
          setDownloadingId(null);
          break;
      }
    };

    try {
      if (modelId === 'parakeet-tdt-v2-fp32') {
        await invoke('download_parakeet_fp32_model', { onEvent });
      } else {
        await invoke('download_model', { modelId, onEvent });
      }
    } catch (e) {
      if (!cancelledRef.current) {
        setErrorMsg(String(e));
        setDownloadState('error');
        setDownloadingId(null);
      }
    }
  }

  function handleCancel() {
    cancelledRef.current = true;
    setDownloadState('idle');
    setDownloadingId(null);
    setDownloadedBytes(0);
    setTotalBytes(0);
    setErrorMsg('');
  }

  function handleRetry() {
    if (downloadingId) {
      handleDownload(downloadingId);
    } else {
      setDownloadState('idle');
    }
  }

  const percent =
    totalBytes > 0 ? Math.round((downloadedBytes / totalBytes) * 100) : null;

  const gridClass =
    visibleModels.length >= 4
      ? 'grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 mb-6'
      : visibleModels.length >= 3
        ? 'grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3 mb-6'
        : visibleModels.length === 2
          ? 'grid grid-cols-1 gap-4 sm:grid-cols-2 mb-6'
          : 'grid grid-cols-1 gap-4 mb-6';

  return (
    <div className="flex h-full w-full flex-col items-center justify-center px-8 py-10 bg-white dark:bg-gray-900">
      <div className="w-full max-w-3xl">
        {/* Header */}
        <div className="mb-8 text-center">
          <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-2">
            Set Up VoiceType
          </h1>
          <p className="text-sm text-gray-500 dark:text-gray-400">
            Offline transcription requires a local AI model file — choose one to download below.
          </p>
        </div>

        {/* GPU detection badge */}
        <div className="mb-6 flex justify-center">
          {gpuDetected ? (
            <span className="inline-flex items-center gap-1.5 rounded-full bg-green-100 px-3 py-1 text-sm font-medium text-green-700 dark:bg-green-900/40 dark:text-green-400">
              <span className="h-2 w-2 rounded-full bg-green-500" />
              {gpuName || 'NVIDIA GPU Detected'}
            </span>
          ) : directmlAvailable ? (
            <span className="inline-flex items-center gap-1.5 rounded-full bg-blue-100 px-3 py-1 text-sm font-medium text-blue-700 dark:bg-blue-900/40 dark:text-blue-400">
              <span className="h-2 w-2 rounded-full bg-blue-500" />
              GPU Detected (DirectML)
            </span>
          ) : (
            <span className="inline-flex items-center gap-1.5 rounded-full bg-gray-100 px-3 py-1 text-sm font-medium text-gray-600 dark:bg-gray-800 dark:text-gray-400">
              <span className="h-2 w-2 rounded-full bg-gray-400" />
              CPU Mode
            </span>
          )}
        </div>

        {/* Model cards */}
        <div className={gridClass}>
          {visibleModels.map((model) => {
            const isRecommended = model.id === recommendedModel;
            const isDownloading = downloadingId === model.id && downloadState === 'downloading';
            const isValidating = downloadingId === model.id && downloadState === 'validating';
            const isComplete = downloadingId === model.id && downloadState === 'complete';
            const isError = downloadingId === model.id && downloadState === 'error';
            const otherDownloading =
              downloadingId !== null &&
              downloadingId !== model.id &&
              (downloadState === 'downloading' || downloadState === 'validating');

            return (
              <div
                key={model.id}
                className={[
                  'relative rounded-xl border-2 p-4 transition-colors',
                  isRecommended
                    ? 'border-indigo-400 bg-indigo-50 dark:border-indigo-500 dark:bg-indigo-950/40'
                    : 'border-gray-200 bg-white dark:border-gray-700 dark:bg-gray-800',
                ].join(' ')}
              >
                {isRecommended && (
                  <span className="absolute -top-2.5 left-3 rounded-full bg-indigo-500 px-2 py-0.5 text-xs font-semibold text-white">
                    Recommended
                  </span>
                )}

                <div className="mb-3">
                  <p className="font-semibold text-gray-900 dark:text-gray-100">{model.name}</p>
                  <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                    {model.size} &middot; {model.quality}
                  </p>
                  <p className="text-xs text-gray-400 dark:text-gray-500 mt-0.5">
                    {model.requirement}
                  </p>
                </div>

                {/* Progress bar */}
                {(isDownloading || isValidating) && (
                  <div className="mb-3">
                    <div className="h-1.5 w-full rounded-full bg-gray-200 dark:bg-gray-700 overflow-hidden">
                      <div
                        className="h-full rounded-full bg-indigo-500 transition-all duration-200"
                        style={{ width: percent !== null ? `${percent}%` : '100%' }}
                      />
                    </div>
                    <p className="mt-1.5 text-xs text-gray-500 dark:text-gray-400">
                      {isValidating
                        ? 'Verifying checksum...'
                        : percent !== null
                          ? `${percent}% — ${formatMB(downloadedBytes)} / ${formatMB(totalBytes)}`
                          : `Downloading... ${formatMB(downloadedBytes)}`}
                    </p>
                  </div>
                )}

                {isComplete && (
                  <p className="mb-3 text-xs font-medium text-green-600 dark:text-green-400">
                    Download complete — enabling autostart...
                  </p>
                )}

                {isError && (
                  <p className="mb-3 text-xs text-red-600 dark:text-red-400 break-all">
                    Error: {errorMsg}
                  </p>
                )}

                {/* Buttons */}
                {isError ? (
                  <button
                    onClick={handleRetry}
                    className="w-full rounded-lg bg-red-500 px-3 py-1.5 text-sm font-medium text-white hover:bg-red-600 transition-colors"
                  >
                    Retry
                  </button>
                ) : isDownloading ? (
                  <button
                    onClick={handleCancel}
                    className="w-full rounded-lg border border-gray-300 dark:border-gray-600 px-3 py-1.5 text-sm font-medium text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                  >
                    Cancel
                  </button>
                ) : !isComplete && !isValidating ? (
                  <button
                    onClick={() => handleDownload(model.id)}
                    disabled={otherDownloading}
                    className={[
                      'w-full rounded-lg px-3 py-1.5 text-sm font-medium transition-colors',
                      isRecommended
                        ? 'bg-indigo-500 text-white hover:bg-indigo-600 disabled:opacity-50 disabled:cursor-not-allowed'
                        : 'border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed',
                    ].join(' ')}
                  >
                    Download
                  </button>
                ) : null}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
