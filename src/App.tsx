import { useEffect, useState } from 'react';
import { HotkeyCapture } from './components/HotkeyCapture';
import { ThemeToggle } from './components/ThemeToggle';
import { AutostartToggle } from './components/AutostartToggle';
import { getStore, DEFAULTS } from './lib/store';

function App() {
  const [hotkey, setHotkey] = useState(DEFAULTS.hotkey);
  const [theme, setTheme] = useState<'light' | 'dark'>(DEFAULTS.theme);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    async function loadSettings() {
      const store = await getStore();
      const savedHotkey = await store.get<string>('hotkey');
      const savedTheme = await store.get<'light' | 'dark'>('theme');

      if (savedHotkey) setHotkey(savedHotkey);

      const resolvedTheme = savedTheme ?? DEFAULTS.theme;
      setTheme(resolvedTheme);

      // Apply theme to DOM
      if (resolvedTheme === 'dark') {
        document.documentElement.classList.add('dark');
      } else {
        document.documentElement.classList.remove('dark');
      }

      setLoaded(true);
    }

    loadSettings();
  }, []);

  if (!loaded) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-white dark:bg-gray-900">
        <div className="text-sm text-gray-400">Loading...</div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-white px-6 py-5 text-gray-900 dark:bg-gray-900 dark:text-gray-100">
      {/* Header */}
      <h1 className="mb-6 text-base font-semibold tracking-tight text-gray-900 dark:text-gray-100">
        VoiceType Settings
      </h1>

      <div className="space-y-6">
        {/* Hotkey section */}
        <section>
          <h2 className="mb-1 text-xs font-medium uppercase tracking-wider text-gray-500 dark:text-gray-400">
            Hotkey
          </h2>
          <p className="mb-2 text-sm text-gray-500 dark:text-gray-400">
            Click the box below then press your desired key combination.
          </p>
          <HotkeyCapture value={hotkey} onChange={setHotkey} />
        </section>

        <hr className="border-gray-200 dark:border-gray-700" />

        {/* Appearance section */}
        <section>
          <h2 className="mb-1 text-xs font-medium uppercase tracking-wider text-gray-500 dark:text-gray-400">
            Appearance
          </h2>
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-900 dark:text-gray-100">Dark mode</p>
              <p className="text-xs text-gray-500 dark:text-gray-400">
                Switch between light and dark interface
              </p>
            </div>
            <ThemeToggle
              theme={theme}
              onChange={(next) => setTheme(next)}
            />
          </div>
        </section>

        <hr className="border-gray-200 dark:border-gray-700" />

        {/* Startup section */}
        <section>
          <h2 className="mb-1 text-xs font-medium uppercase tracking-wider text-gray-500 dark:text-gray-400">
            Startup
          </h2>
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-900 dark:text-gray-100">Launch at login</p>
              <p className="text-xs text-gray-500 dark:text-gray-400">
                Start VoiceType automatically when Windows starts
              </p>
            </div>
            <AutostartToggle />
          </div>
        </section>
      </div>
    </div>
  );
}

export default App;
