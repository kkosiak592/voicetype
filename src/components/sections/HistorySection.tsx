import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Copy, Check } from 'lucide-react';

interface HistoryEntry {
  text: string;
  timestampMs: number;
  engine: string;
}

export function HistorySection() {
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);

  const loadHistory = () => {
    invoke<HistoryEntry[]>('get_history')
      .then(setEntries)
      .catch((err) => console.error('Failed to load history:', err));
  };

  useEffect(() => {
    loadHistory();

    const unlistenPromise = listen('history-updated', () => {
      loadHistory();
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  function handleCopy(text: string, index: number) {
    navigator.clipboard.writeText(text).then(() => {
      setCopiedIndex(index);
      setTimeout(() => setCopiedIndex(null), 1500);
    }).catch(() => { });
  }

  function formatTimestamp(ms: number): string {
    return new Date(ms).toLocaleString(undefined, {
      month: 'short',
      day: 'numeric',
      hour: 'numeric',
      minute: '2-digit',
    });
  }

  return (
    <div className="flex h-full flex-col">
      <div className="mb-4 shrink-0">
        <h1 className="text-xl font-bold tracking-tight text-gray-900 dark:text-gray-100">
          History
        </h1>
        <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
          Review and copy your recent transcriptions.
        </p>
      </div>

      <div className="flex min-h-0 flex-1 flex-col rounded-2xl bg-white pt-1 pb-2 ring-1 shadow-sm ring-gray-200 dark:bg-gray-900 dark:ring-gray-800">
        {entries.length === 0 ? (
          <div className="flex flex-1 items-center justify-center p-8 text-center">
            <p className="text-sm text-gray-500 dark:text-gray-400">
              No transcription history yet.<br />Dictate something to see it here.
            </p>
          </div>
        ) : (
          <div className="flex-1 space-y-3 overflow-y-auto px-4 py-3">
            {entries.map((entry, index) => (
              <button
                key={index}
                type="button"
                onClick={() => handleCopy(entry.text, index)}
                className="group relative flex w-full flex-col gap-2 rounded-xl border border-gray-100 bg-gray-50/50 p-4 text-left transition-all hover:border-emerald-200 hover:bg-emerald-50/30 hover:shadow-sm focus:outline-none focus:ring-2 focus:ring-emerald-500/50 dark:border-gray-800 dark:bg-gray-800/30 dark:hover:border-emerald-900/50 dark:hover:bg-emerald-900/10"
              >
                <div className="flex w-full items-start justify-between gap-4">
                  <p className="text-sm leading-relaxed text-gray-700 dark:text-gray-300">
                    {entry.text}
                  </p>
                  <div
                    className="shrink-0 rounded-lg p-2 text-gray-400 transition-colors group-hover:bg-white group-hover:text-emerald-600 group-hover:shadow-sm dark:group-hover:bg-gray-800 dark:group-hover:text-emerald-400"
                    title="Copy to clipboard"
                  >
                    {copiedIndex === index ? (
                      <Check className="size-4 text-emerald-600 dark:text-emerald-400" />
                    ) : (
                      <Copy className="size-4" />
                    )}
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-[10px] font-semibold uppercase tracking-wider text-gray-400 dark:text-gray-500">
                    {formatTimestamp(entry.timestampMs)}
                  </span>
                  <span className="size-1 shrink-0 rounded-full bg-gray-300 dark:bg-gray-600"></span>
                  <span className="text-[10px] font-semibold uppercase tracking-wider text-gray-400 dark:text-gray-500">
                    {entry.engine}
                  </span>
                </div>
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
