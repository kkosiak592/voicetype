import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface HistoryEntry {
  text: string;
  timestampMs: number;
  engine: string;
}

export function HistorySection() {
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);

  useEffect(() => {
    invoke<HistoryEntry[]>('get_history')
      .then(setEntries)
      .catch((err) => console.error('Failed to load history:', err));
  }, []);

  function handleCopy(text: string, index: number) {
    navigator.clipboard.writeText(text).then(() => {
      setCopiedIndex(index);
      setTimeout(() => setCopiedIndex(null), 1500);
    }).catch(() => {});
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
    <div className="space-y-4">
      <h2 className="text-base font-semibold tracking-tight text-gray-900 dark:text-gray-100">
        History
      </h2>

      {entries.length === 0 ? (
        <p className="text-sm text-gray-500 dark:text-gray-400">
          No transcription history yet. Dictate something to see it here.
        </p>
      ) : (
        <div className="max-h-[calc(100vh-140px)] overflow-y-auto space-y-0.5">
          {entries.map((entry, index) => (
            <button
              key={index}
              type="button"
              title={entry.text}
              onClick={() => handleCopy(entry.text, index)}
              className="w-full text-left px-3 py-2.5 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 cursor-pointer transition-colors"
            >
              <div className="flex items-start justify-between gap-2">
                <span className="text-sm text-gray-900 dark:text-gray-100 flex-1 truncate">
                  {entry.text.length > 100
                    ? entry.text.slice(0, 100) + '…'
                    : entry.text}
                </span>
                {copiedIndex === index && (
                  <span className="shrink-0 text-xs text-green-600 dark:text-green-400 font-medium">
                    Copied!
                  </span>
                )}
              </div>
              <div className="mt-0.5 flex items-center gap-2">
                <span className="text-xs text-gray-500 dark:text-gray-400">
                  {formatTimestamp(entry.timestampMs)}
                </span>
                <span className="inline-block px-1.5 py-0.5 rounded text-xs bg-gray-100 dark:bg-gray-700 text-gray-500 dark:text-gray-400">
                  {entry.engine}
                </span>
              </div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
