import { getStore } from '../lib/store';

interface ThemeToggleProps {
  theme: 'light' | 'dark';
  onChange: (theme: 'light' | 'dark') => void;
}

export function ThemeToggle({ theme, onChange }: ThemeToggleProps) {
  const isDark = theme === 'dark';

  async function handleToggle() {
    const next: 'light' | 'dark' = isDark ? 'light' : 'dark';

    // Apply to DOM
    if (next === 'dark') {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }

    // Persist
    const store = await getStore();
    await store.set('theme', next);

    onChange(next);
  }

  return (
    <button
      onClick={handleToggle}
      className={[
        'relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:ring-offset-2 dark:focus:ring-offset-gray-900',
        isDark
          ? 'bg-emerald-500'
          : 'bg-gray-200 dark:bg-gray-700',
      ].join(' ')}
      role="switch"
      aria-checked={isDark}
    >
      <span className="sr-only">Toggle theme</span>
      <span
        aria-hidden="true"
        className={[
          'pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out',
          isDark ? 'translate-x-5' : 'translate-x-0',
        ].join(' ')}
      />
    </button>
  );
}
