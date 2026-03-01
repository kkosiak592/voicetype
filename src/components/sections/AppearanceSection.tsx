import { ThemeToggle } from '../ThemeToggle';
import { AutostartToggle } from '../AutostartToggle';

interface AppearanceSectionProps {
  theme: 'light' | 'dark';
  onThemeChange: (theme: 'light' | 'dark') => void;
}

export function AppearanceSection({ theme, onThemeChange }: AppearanceSectionProps) {
  return (
    <div>
      <h1 className="mb-5 text-base font-semibold tracking-tight text-gray-900 dark:text-gray-100">
        Appearance
      </h1>

      <div className="space-y-5">
        {/* Dark mode */}
        <section>
          <h2 className="mb-1 text-xs font-medium uppercase tracking-wider text-gray-500 dark:text-gray-400">
            Theme
          </h2>
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-900 dark:text-gray-100">Dark mode</p>
              <p className="text-xs text-gray-500 dark:text-gray-400">
                Switch between light and dark interface
              </p>
            </div>
            <ThemeToggle theme={theme} onChange={onThemeChange} />
          </div>
        </section>

        <hr className="border-gray-200 dark:border-gray-700" />

        {/* Launch at login */}
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
