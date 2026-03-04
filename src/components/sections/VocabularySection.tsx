import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { DictionaryEditor } from '../DictionaryEditor';

export function VocabularySection() {
  const [prompt, setPrompt] = useState('');
  const [corrections, setCorrections] = useState<Record<string, string>>({});
  const [allCaps, setAllCaps] = useState(false);
  const [loading, setLoading] = useState(true);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    async function loadInitial() {
      const [savedPrompt, correctionMap, savedAllCaps] = await Promise.all([
        invoke<string>('get_vocabulary_prompt'),
        invoke<Record<string, string>>('get_corrections'),
        invoke<boolean>('get_all_caps'),
      ]);
      setPrompt(savedPrompt);
      setCorrections(correctionMap);
      setAllCaps(savedAllCaps);
      setLoading(false);
    }
    loadInitial().catch(err => {
      console.error('Failed to load vocabulary settings:', err);
      setLoading(false);
    });
  }, []);

  function handlePromptChange(value: string) {
    setPrompt(value);
    // Debounce: save after 1 second of no typing
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => {
      invoke('set_vocabulary_prompt', { prompt: value }).catch(err => {
        console.error('Failed to save vocabulary prompt:', err);
      });
    }, 1000);
  }

  function handlePromptBlur() {
    // Also save immediately on blur
    if (debounceRef.current) clearTimeout(debounceRef.current);
    invoke('set_vocabulary_prompt', { prompt }).catch(err => {
      console.error('Failed to save vocabulary prompt on blur:', err);
    });
  }

  async function handleAllCapsToggle() {
    const next = !allCaps;
    try {
      await invoke('set_all_caps', { enabled: next });
      setAllCaps(next);
    } catch (err) {
      console.error('Failed to toggle ALL CAPS:', err);
    }
  }

  async function handleCorrectionsChange(updated: Record<string, string>) {
    try {
      await invoke('save_corrections', { corrections: updated });
      setCorrections(updated);
    } catch (err) {
      console.error('Failed to save corrections:', err);
    }
  }

  if (loading) {
    return (
      <div>
        <h1 className="mb-5 text-base font-semibold tracking-tight text-gray-900 dark:text-gray-100">
          Vocabulary
        </h1>
        <div className="space-y-2">
          <div className="h-24 animate-pulse rounded-lg bg-gray-100 dark:bg-gray-800" />
        </div>
      </div>
    );
  }

  return (
    <div>
      <h1 className="mb-4 text-base font-semibold tracking-tight text-gray-900 dark:text-gray-100">
        Vocabulary
      </h1>

      <div className="space-y-4">
        {/* Vocabulary prompt */}
        <div>
          <label className="mb-1.5 block text-sm font-medium text-gray-900 dark:text-gray-100">
            Vocabulary Prompt
          </label>
          <textarea
            value={prompt}
            onChange={(e) => handlePromptChange(e.target.value)}
            onBlur={handlePromptBlur}
            rows={4}
            placeholder="Enter domain-specific terms to improve recognition accuracy..."
            className="w-full rounded border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:border-indigo-400 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 dark:placeholder-gray-500 resize-none"
          />
          <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
            Terms and phrases injected as context to bias the model toward domain vocabulary.
          </p>
        </div>

        {/* ALL CAPS toggle */}
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm font-medium text-gray-900 dark:text-gray-100">ALL CAPS output</p>
            <p className="text-xs text-gray-500 dark:text-gray-400">
              Uppercase all injected text
            </p>
          </div>
          <button
            onClick={handleAllCapsToggle}
            className={[
              'relative inline-flex h-6 w-11 items-center rounded-full transition-colors duration-200 focus:outline-none',
              allCaps ? 'bg-blue-600' : 'bg-gray-300 dark:bg-gray-600',
            ].join(' ')}
            role="switch"
            aria-checked={allCaps}
          >
            <span
              className={[
                'inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform duration-200',
                allCaps ? 'translate-x-6' : 'translate-x-1',
              ].join(' ')}
            />
          </button>
        </div>

        <hr className="border-gray-200 dark:border-gray-700" />

        {/* Corrections dictionary */}
        <section>
          <h2 className="mb-2 text-xs font-medium uppercase tracking-wider text-gray-500 dark:text-gray-400">
            Corrections Dictionary
          </h2>
          <DictionaryEditor corrections={corrections} onChange={handleCorrectionsChange} />
        </section>
      </div>
    </div>
  );
}
