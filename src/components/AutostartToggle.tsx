import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart';
import { useEffect, useState } from 'react';
import { getStore } from '../lib/store';

export function AutostartToggle() {
  const [enabled, setEnabled] = useState(false);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    isEnabled().then((val) => {
      setEnabled(val);
      setLoading(false);
    }).catch(err => {
      console.error('Failed to check autostart:', err);
      setLoading(false);
    });
  }, []);

  async function handleToggle() {
    const next = !enabled;
    if (next) {
      await enable();
    } else {
      await disable();
    }

    // Persist UI state
    const store = await getStore();
    await store.set('autostart', next);

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
        'relative inline-flex h-6 w-11 items-center rounded-full transition-colors duration-200 focus:outline-none',
        enabled
          ? 'bg-blue-600'
          : 'bg-gray-300 dark:bg-gray-600',
      ].join(' ')}
      role="switch"
      aria-checked={enabled}
    >
      <span
        className={[
          'inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform duration-200',
          enabled ? 'translate-x-6' : 'translate-x-1',
        ].join(' ')}
      />
    </button>
  );
}
