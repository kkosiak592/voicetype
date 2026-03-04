import { ThemeToggle } from '../ThemeToggle';
import { AutostartToggle } from '../AutostartToggle';

interface AppearanceSectionProps {
  theme: 'light' | 'dark';
  onThemeChange: (theme: 'light' | 'dark') => void;
}

export function AppearanceSection({ theme, onThemeChange }: AppearanceSectionProps) {
  return (
    <div>
      <div className="mb-4">
        <h1 className="text-xl font-bold tracking-tight text-gray-900 dark:text-gray-100">
          Appearance
        </h1>
        <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
          Customize the look and feel of VoiceType.
        </p>
      </div>

      <div className="space-y-4">
        <div className="bg-white dark:bg-gray-900 ring-1 ring-gray-200 dark:ring-gray-800 rounded-2xl p-4 shadow-sm">
          {/* Theme */}
          <section>
            <h2 className="text-sm font-semibold text-gray-900 dark:text-gray-100 mb-3">
              Theme Options
            </h2>
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-medium text-gray-900 dark:text-gray-100">Dark mode</p>
                <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                  Switch between light and dark interface
                </p>
              </div>
              <ThemeToggle theme={theme} onChange={onThemeChange} />
            </div>
          </section>

          <div className="my-4 border-t border-gray-100 dark:border-gray-800" />

          {/* Launch at login */}
          <section>
            <h2 className="text-sm font-semibold text-gray-900 dark:text-gray-100 mb-3">
              System Integration
            </h2>
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-medium text-gray-900 dark:text-gray-100">Launch at login</p>
                <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                  Start VoiceType automatically when Windows starts
                </p>
              </div>
              <AutostartToggle />
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
