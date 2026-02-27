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
        'relative inline-flex h-6 w-11 items-center rounded-full transition-colors duration-200 focus:outline-none',
        isDark
          ? 'bg-blue-600'
          : 'bg-gray-300 dark:bg-gray-600',
      ].join(' ')}
      role="switch"
      aria-checked={isDark}
    >
      <span
        className={[
          'inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform duration-200',
          isDark ? 'translate-x-6' : 'translate-x-1',
        ].join(' ')}
      />
    </button>
  );
}
