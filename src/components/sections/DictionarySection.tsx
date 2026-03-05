import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { DictionaryEditor } from '../DictionaryEditor';

export function DictionarySection() {
  const [corrections, setCorrections] = useState<Record<string, string>>({});

  useEffect(() => {
    invoke<Record<string, string>>('get_corrections')
      .then((data) => setCorrections(data))
      .catch((err) => console.error('Failed to load corrections:', err));
  }, []);

  async function handleCorrectionsChange(updated: Record<string, string>) {
    setCorrections(updated);
    try {
      await invoke('save_corrections', { corrections: updated });
    } catch (err) {
      console.error('Failed to save corrections:', err);
    }
  }

  return (
    <div>
      <div className="mb-4">
        <h1 className="text-xl font-bold tracking-tight text-gray-900 dark:text-gray-100">
          Corrections Dictionary
        </h1>
        <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
          Fix recurring transcription mistakes. Matched words are automatically replaced.
        </p>
      </div>

      <div className="bg-white dark:bg-gray-900 ring-1 ring-gray-200 dark:ring-gray-800 rounded-2xl p-4 shadow-sm">
        <DictionaryEditor corrections={corrections} onChange={handleCorrectionsChange} />
      </div>
    </div>
  );
}
