import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Copy, Check, Pencil, X } from 'lucide-react';

interface HistoryEntry {
  text: string;
  timestampMs: number;
  engine: string;
  rawText?: string;
}

interface PromotedCorrection {
  from: string;
  to: string;
}

/**
 * Simple positional word-level diff between two strings.
 *
 * Splits both by whitespace and compares at the same index. Returns pairs where
 * the raw word differs from the corrected word. Unmatched tail (length mismatch)
 * is ignored — covers 90% case of word substitutions which is what the
 * corrections dictionary handles.
 */
function extractWordDiffs(raw: string, corrected: string): Array<{ from: string; to: string }> {
  const rawWords = raw.trim().split(/\s+/);
  const correctedWords = corrected.trim().split(/\s+/);
  const minLen = Math.min(rawWords.length, correctedWords.length);
  const diffs: Array<{ from: string; to: string }> = [];

  for (let i = 0; i < minLen; i++) {
    if (rawWords[i] !== correctedWords[i]) {
      diffs.push({ from: rawWords[i], to: correctedWords[i] });
    }
  }

  return diffs;
}

export function HistorySection() {
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);
  const [editingIndex, setEditingIndex] = useState<number | null>(null);
  const [editText, setEditText] = useState('');
  const [notification, setNotification] = useState<PromotedCorrection | null>(null);
  const notificationTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

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

  function openEdit(index: number, rawText: string) {
    setEditingIndex(index);
    setEditText(rawText);
  }

  function closeEdit() {
    setEditingIndex(null);
    setEditText('');
  }

  function showNotification(promoted: PromotedCorrection) {
    // Clear existing timer before setting a new one
    if (notificationTimerRef.current !== null) {
      clearTimeout(notificationTimerRef.current);
    }
    setNotification(promoted);
    notificationTimerRef.current = setTimeout(() => {
      setNotification(null);
      notificationTimerRef.current = null;
    }, 10000);
  }

  function dismissNotification() {
    if (notificationTimerRef.current !== null) {
      clearTimeout(notificationTimerRef.current);
      notificationTimerRef.current = null;
    }
    setNotification(null);
  }

  async function handleUndo(from: string, to: string) {
    try {
      await invoke('undo_promotion', { from, to });
    } catch (err) {
      console.error('Failed to undo promotion:', err);
    }
    dismissNotification();
  }

  async function handleSubmitCorrection(entry: HistoryEntry) {
    if (editingIndex === null || !entry.rawText) return;

    const diffs = extractWordDiffs(entry.rawText, editText);
    closeEdit();

    for (const diff of diffs) {
      try {
        const promoted = await invoke<PromotedCorrection | null>('submit_correction', {
          from: diff.from,
          to: diff.to,
        });
        if (promoted) {
          showNotification(promoted);
        }
      } catch (err) {
        console.error('Failed to submit correction:', err);
      }
    }
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

      {/* Auto-promote notification banner */}
      {notification && (
        <div className="mb-3 flex items-center justify-between rounded-xl bg-emerald-50 px-4 py-2.5 dark:bg-emerald-900/20">
          <span className="text-sm text-emerald-700 dark:text-emerald-300">
            Auto-added to dictionary: <strong>{notification.from}</strong> &rarr; <strong>{notification.to}</strong>
          </span>
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => handleUndo(notification.from, notification.to)}
              className="text-xs font-medium text-emerald-700 underline hover:no-underline dark:text-emerald-300"
            >
              Undo
            </button>
            <button
              type="button"
              onClick={dismissNotification}
              className="rounded p-0.5 text-emerald-600 hover:bg-emerald-100 dark:text-emerald-400 dark:hover:bg-emerald-900/40"
              title="Dismiss"
            >
              <X className="size-3.5" />
            </button>
          </div>
        </div>
      )}

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
              <div
                key={index}
                className="group relative flex w-full flex-col gap-2 rounded-xl border border-gray-100 bg-gray-50/50 p-4 dark:border-gray-800 dark:bg-gray-800/30"
              >
                {/* Top row: text + action buttons */}
                <div className="flex w-full items-start justify-between gap-4">
                  <p className="text-sm leading-relaxed text-gray-700 dark:text-gray-300">
                    {entry.text}
                  </p>
                  <div className="flex shrink-0 items-center gap-1">
                    {/* Correction edit button — only when corrections were applied */}
                    {entry.rawText && editingIndex !== index && (
                      <button
                        type="button"
                        onClick={() => openEdit(index, entry.rawText!)}
                        className="rounded-lg p-2 text-gray-400 transition-colors hover:bg-white hover:text-blue-600 hover:shadow-sm dark:hover:bg-gray-800 dark:hover:text-blue-400"
                        title="Correct raw transcription"
                      >
                        <Pencil className="size-4" />
                      </button>
                    )}
                    {/* Copy button */}
                    <button
                      type="button"
                      onClick={() => handleCopy(entry.text, index)}
                      className="rounded-lg p-2 text-gray-400 transition-colors hover:bg-white hover:text-emerald-600 hover:shadow-sm dark:hover:bg-gray-800 dark:hover:text-emerald-400"
                      title="Copy to clipboard"
                    >
                      {copiedIndex === index ? (
                        <Check className="size-4 text-emerald-600 dark:text-emerald-400" />
                      ) : (
                        <Copy className="size-4" />
                      )}
                    </button>
                  </div>
                </div>

                {/* Metadata row */}
                <div className="flex items-center gap-2">
                  <span className="text-[10px] font-semibold uppercase tracking-wider text-gray-400 dark:text-gray-500">
                    {formatTimestamp(entry.timestampMs)}
                  </span>
                  <span className="size-1 shrink-0 rounded-full bg-gray-300 dark:bg-gray-600"></span>
                  <span className="text-[10px] font-semibold uppercase tracking-wider text-gray-400 dark:text-gray-500">
                    {entry.engine}
                  </span>
                </div>

                {/* Inline correction editor */}
                {editingIndex === index && entry.rawText && (
                  <div className="mt-1 flex flex-col gap-2 rounded-lg border border-blue-200 bg-blue-50/50 p-3 dark:border-blue-900/50 dark:bg-blue-900/10">
                    <p className="text-[11px] font-medium uppercase tracking-wider text-blue-600 dark:text-blue-400">
                      Correct raw transcription
                    </p>
                    <textarea
                      value={editText}
                      onChange={(e) => setEditText(e.target.value)}
                      rows={3}
                      className="w-full resize-none rounded-lg border border-gray-200 bg-white px-3 py-2 text-sm text-gray-800 outline-none focus:border-blue-400 focus:ring-1 focus:ring-blue-400 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-200"
                      placeholder="Type the corrected version..."
                    />
                    <div className="flex gap-2">
                      <button
                        type="button"
                        onClick={() => handleSubmitCorrection(entry)}
                        className="rounded-lg bg-blue-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-blue-700"
                      >
                        Submit Correction
                      </button>
                      <button
                        type="button"
                        onClick={closeEdit}
                        className="rounded-lg border border-gray-200 px-3 py-1.5 text-xs font-medium text-gray-600 hover:bg-gray-100 dark:border-gray-700 dark:text-gray-400 dark:hover:bg-gray-800"
                      >
                        Cancel
                      </button>
                    </div>
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
