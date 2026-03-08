import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';
import { store } from '../lib/store';

export function PrefixTextInput() {
  const [enabled, setEnabled] = useState(false);
  const [text, setText] = useState('');
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    Promise.all([
      store.get<boolean>('prefix_enabled'),
      store.get<string>('prefix_text'),
    ])
      .then(([prefixEnabled, prefixText]) => {
        setEnabled(prefixEnabled ?? false);
        setText(prefixText ?? '');
        setLoading(false);
      })
      .catch(() => {
        setLoading(false);
      });
  }, []);

  async function handleToggle() {
    const next = !enabled;
    await invoke('set_prefix_enabled', { enabled: next });
    setEnabled(next);
  }

  async function handleTextChange(e: React.ChangeEvent<HTMLInputElement>) {
    const newValue = e.target.value;
    setText(newValue);
    await invoke('set_prefix_text', { text: newValue });
  }

  if (loading) {
    return (
      <div className="h-6 w-11 animate-pulse rounded-full bg-gray-200 dark:bg-gray-600" />
    );
  }

  return (
    <div>
      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm font-medium text-gray-900 dark:text-gray-100">Prefix Text</p>
          <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
            Prepend a custom string to all dictated output
          </p>
        </div>
        <button
          onClick={handleToggle}
          className={[
            'relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:ring-offset-2 dark:focus:ring-offset-gray-900',
            enabled
              ? 'bg-emerald-500'
              : 'bg-gray-200 dark:bg-gray-700',
          ].join(' ')}
          role="switch"
          aria-checked={enabled}
        >
          <span className="sr-only">Toggle Prefix Text</span>
          <span
            aria-hidden="true"
            className={[
              'pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out',
              enabled ? 'translate-x-5' : 'translate-x-0',
            ].join(' ')}
          />
        </button>
      </div>
      {enabled && (
        <input
          type="text"
          value={text}
          onChange={handleTextChange}
          placeholder="e.g., TEPC: "
          className="mt-3 w-full rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-sm text-gray-900 dark:text-gray-100 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:border-emerald-500 dark:focus:ring-emerald-500 dark:focus:border-emerald-500 placeholder-gray-400 dark:placeholder-gray-500"
        />
      )}
    </div>
  );
}
