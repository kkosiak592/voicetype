import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';
import { store } from '../lib/store';

export function AllCapsToggle() {
  const [enabled, setEnabled] = useState(false);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    // Read from settings store (safe before setup — returns null/false).
    // Avoids calling get_all_caps which accesses ActiveProfile state
    // that isn't manage()'d until deep in setup().
    store.get<boolean>('all_caps').then((val) => {
      setEnabled(val ?? false);
      setLoading(false);
    }).catch(() => {
      setLoading(false);
    });
  }, []);

  async function handleToggle() {
    const next = !enabled;
    await invoke('set_all_caps', { enabled: next });
    setEnabled(next);
  }

  if (loading) {
    return (
      <div className="h-6 w-11 animate-pulse rounded-full bg-gray-200 dark:bg-gray-600" />
    );
  }

  return (
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
      <span className="sr-only">Toggle ALL CAPS</span>
      <span
        aria-hidden="true"
        className={[
          'pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out',
          enabled ? 'translate-x-5' : 'translate-x-0',
        ].join(' ')}
      />
    </button>
  );
}
