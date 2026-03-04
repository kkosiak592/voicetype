import type { UpdateState } from '../lib/updater';

interface UpdateBannerProps {
  state: UpdateState;
  onDownload: () => void;
  onCancel: () => void;
  onRestart: () => void;
  onLater: () => void;
  onDismiss: () => void;
  onRetry: () => void;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function Spinner() {
  return (
    <svg
      className="h-3.5 w-3.5 animate-spin text-gray-400"
      xmlns="http://www.w3.org/2000/svg"
      fill="none"
      viewBox="0 0 24 24"
    >
      <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
      <path
        className="opacity-75"
        fill="currentColor"
        d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
      />
    </svg>
  );
}

export function UpdateBanner({
  state,
  onDownload,
  onCancel,
  onRestart,
  onLater,
  onDismiss,
  onRetry,
}: UpdateBannerProps) {
  const { status, version, body, downloaded, contentLength, errorMessage, dismissed } = state;

  // Hide banner when idle or dismissed
  if (status === 'idle' || dismissed) return null;

  // Checking: minimal status bar
  if (status === 'checking') {
    return (
      <div className="absolute inset-x-0 top-0 z-50 flex items-center gap-2 border-b border-gray-100 bg-gray-50 px-4 py-2 text-sm text-gray-500 shadow-sm dark:border-gray-700 dark:bg-gray-800 dark:text-gray-400">
        <Spinner />
        <span>Checking for updates...</span>
      </div>
    );
  }

  // Available: emerald banner with version, release notes, download button
  if (status === 'available') {
    const releaseNotesPreview = body
      .split('\n')
      .filter(line => line.trim().length > 0)
      .slice(0, 3)
      .join(' · ');

    return (
      <div className="absolute inset-x-0 top-0 z-50 border-b border-emerald-100 bg-emerald-50 px-4 py-3 shadow-sm dark:border-emerald-900 dark:bg-emerald-950">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0 flex-1">
            <p className="text-sm font-medium text-emerald-900 dark:text-emerald-100">
              VoiceType {version} is available
            </p>
            {releaseNotesPreview && (
              <p className="mt-0.5 truncate text-xs text-emerald-700 dark:text-emerald-300">
                {releaseNotesPreview}
              </p>
            )}
          </div>
          <div className="flex shrink-0 items-center gap-2">
            <button
              onClick={onDownload}
              className="rounded bg-emerald-600 px-3 py-1 text-xs font-medium text-white hover:bg-emerald-700 focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:ring-offset-1 dark:bg-emerald-500 dark:hover:bg-emerald-400"
            >
              Download
            </button>
            <button
              onClick={onDismiss}
              aria-label="Dismiss update notification"
              className="rounded p-0.5 text-emerald-400 hover:text-emerald-600 focus:outline-none dark:text-emerald-400 dark:hover:text-emerald-200"
            >
              <svg className="h-4 w-4" viewBox="0 0 20 20" fill="currentColor">
                <path
                  fillRule="evenodd"
                  d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z"
                  clipRule="evenodd"
                />
              </svg>
            </button>
          </div>
        </div>
      </div>
    );
  }

  // Downloading: progress bar with percentage and cancel button
  if (status === 'downloading') {
    const percent = contentLength > 0 ? Math.round((downloaded / contentLength) * 100) : 0;
    const downloadedFmt = formatBytes(downloaded);
    const totalFmt = contentLength > 0 ? formatBytes(contentLength) : '…';

    return (
      <div className="absolute inset-x-0 top-0 z-50 border-b border-emerald-100 bg-emerald-50 px-4 py-3 shadow-sm dark:border-emerald-900 dark:bg-emerald-950">
        <div className="flex items-center justify-between gap-3">
          <div className="min-w-0 flex-1">
            <div className="mb-1.5 flex items-center justify-between">
              <span className="text-xs text-emerald-700 dark:text-emerald-300">
                Downloading update...{' '}
                {contentLength > 0 ? (
                  <span className="font-medium">{percent}%</span>
                ) : (
                  <span>{downloadedFmt}</span>
                )}
                {contentLength > 0 && (
                  <span className="ml-1 text-emerald-500 dark:text-emerald-400">
                    — {downloadedFmt} / {totalFmt}
                  </span>
                )}
              </span>
            </div>
            <div className="h-1.5 w-full overflow-hidden rounded-full bg-emerald-200 dark:bg-emerald-800">
              <div
                className="h-1.5 rounded-full bg-emerald-500 transition-all duration-200"
                style={{ width: contentLength > 0 ? `${percent}%` : '0%' }}
              />
            </div>
          </div>
          <button
            onClick={onCancel}
            className="shrink-0 text-xs text-emerald-500 hover:text-emerald-700 focus:outline-none dark:text-emerald-400 dark:hover:text-emerald-200"
          >
            Cancel
          </button>
        </div>
      </div>
    );
  }

  // Ready: green banner with restart/later options
  if (status === 'ready') {
    return (
      <div className="absolute inset-x-0 top-0 z-50 border-b border-green-100 bg-green-50 px-4 py-3 shadow-sm dark:border-green-900 dark:bg-green-950">
        <div className="flex items-center justify-between gap-3">
          <div className="min-w-0 flex-1">
            <p className="text-sm font-medium text-green-900 dark:text-green-100">
              Update ready — Restart to apply VoiceType {version}
            </p>
            {errorMessage && (
              <p className="mt-0.5 text-xs text-green-700 dark:text-green-300">{errorMessage}</p>
            )}
          </div>
          <div className="flex shrink-0 items-center gap-3">
            <button
              onClick={onRestart}
              className="rounded bg-emerald-600 px-3 py-1 text-xs font-medium text-white hover:bg-emerald-700 focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:ring-offset-1 dark:bg-emerald-500 dark:hover:bg-emerald-400"
            >
              Restart Now
            </button>
            <button
              onClick={onLater}
              className="text-xs text-green-700 hover:underline focus:outline-none dark:text-green-300"
            >
              Later
            </button>
          </div>
        </div>
      </div>
    );
  }

  // Error: red banner with message and retry button
  if (status === 'error') {
    return (
      <div className="absolute inset-x-0 top-0 z-50 border-b border-red-100 bg-red-50 px-4 py-3 shadow-sm dark:border-red-900 dark:bg-red-950">
        <div className="flex items-center justify-between gap-3">
          <div className="min-w-0 flex-1">
            <p className="text-sm font-medium text-red-900 dark:text-red-100">Update check failed</p>
            {errorMessage && (
              <p className="mt-0.5 truncate text-xs text-red-700 dark:text-red-300">{errorMessage}</p>
            )}
          </div>
          <button
            onClick={onRetry}
            className="shrink-0 rounded border border-red-300 bg-white px-3 py-1 text-xs font-medium text-red-700 hover:bg-red-50 focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-offset-1 dark:border-red-700 dark:bg-red-900 dark:text-red-300 dark:hover:bg-red-800"
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  return null;
}
