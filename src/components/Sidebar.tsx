import { Settings, Cpu, Mic, Palette, Clock, Monitor, RefreshCw, type LucideIcon } from 'lucide-react';
import { twMerge } from 'tailwind-merge';
import { useEffect, useState } from 'react';
import { getVersion } from '@tauri-apps/api/app';
import type { UpdateState } from '../lib/updater';

export type SectionId = 'general' | 'model' | 'appearance' | 'system' | 'history';

interface SidebarItem {
  id: SectionId;
  label: string;
  icon: LucideIcon;
}

const ITEMS: SidebarItem[] = [
  { id: 'general', label: 'General', icon: Settings },
  { id: 'model', label: 'Model', icon: Cpu },
  { id: 'appearance', label: 'Appearance', icon: Palette },
  { id: 'system', label: 'System', icon: Monitor },
  { id: 'history', label: 'History', icon: Clock },
];

interface SidebarProps {
  activeSection: SectionId;
  onSelect: (id: SectionId) => void;
  updaterState: UpdateState;
  onCheckForUpdates: () => Promise<void>;
}

export function Sidebar({ activeSection, onSelect, updaterState, onCheckForUpdates }: SidebarProps) {
  const [appVersion, setAppVersion] = useState<string>('');

  useEffect(() => {
    getVersion().then(setAppVersion).catch(() => setAppVersion(''));
  }, []);

  const { status, lastChecked } = updaterState;
  const isChecking = status === 'checking';
  const hasUpdate = status === 'available' || status === 'ready';
  const isUpToDate = status === 'idle' && lastChecked > 0;

  return (
    <nav className="flex w-56 flex-col pt-8 px-4 bg-transparent border-r border-transparent">
      <div className="mb-8 px-2 flex items-center gap-2.5 opacity-90">
        <div className="size-7 bg-emerald-600 rounded-xl shadow-inner shadow-emerald-400/20 flex items-center justify-center">
          <Mic className="size-4 text-white" />
        </div>
        <span className="font-semibold text-sm tracking-wide text-gray-900 dark:text-gray-100">VoiceType</span>
      </div>

      <div className="space-y-1">
        {ITEMS.map((item) => {
          const isActive = activeSection === item.id;
          const Icon = item.icon;

          return (
            <button
              key={item.id}
              onClick={() => onSelect(item.id)}
              className={twMerge(
                'w-full flex items-center gap-3 px-3 py-2.5 text-sm font-medium transition-all duration-200 focus:outline-none rounded-xl text-left group',
                isActive
                  ? 'bg-white dark:bg-gray-800 shadow-sm ring-1 ring-gray-900/5 dark:ring-white/10 text-emerald-600 dark:text-emerald-400'
                  : 'text-gray-600 dark:text-gray-400 hover:bg-gray-200/50 dark:hover:bg-gray-800/50 hover:text-gray-900 dark:hover:text-gray-200'
              )}
            >
              <Icon
                className={twMerge(
                  "size-4.5 transition-transform duration-200",
                  isActive ? "scale-110" : "group-hover:scale-110"
                )}
                strokeWidth={isActive ? 2.5 : 2}
                aria-hidden="true"
              />
              <span>{item.label}</span>
            </button>
          );
        })}
      </div>

      <div className="mt-auto pt-6 pb-6">
        <div className="rounded-xl bg-gray-100/80 p-3 ring-1 ring-gray-200/50 dark:bg-gray-800/50 dark:ring-gray-700/50">
          <div className="mb-2.5 flex items-center justify-between">
            <span className="text-xs font-semibold text-gray-900 dark:text-gray-100">Updates</span>
            {appVersion && <span className="font-mono text-[10px] text-gray-500">v{appVersion}</span>}
          </div>

          {isChecking ? (
            <button disabled className="flex w-full cursor-not-allowed items-center justify-center gap-2 rounded-lg bg-gray-200 px-3 py-1.5 text-xs font-medium text-gray-500 dark:bg-gray-700/80 dark:text-gray-400">
              <RefreshCw className="size-3.5 animate-spin" />
              Checking
            </button>
          ) : hasUpdate ? (
            <span className="flex w-full items-center justify-center rounded-lg bg-emerald-100 px-3 py-1.5 text-xs font-medium text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400">
              Update available
            </span>
          ) : (
            <button
              onClick={onCheckForUpdates}
              className="flex w-full items-center justify-center gap-2 rounded-lg bg-white px-3 py-1.5 text-xs font-medium text-gray-700 shadow-sm ring-1 ring-gray-200 transition-colors hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-emerald-500/50 dark:bg-gray-800 dark:text-gray-300 dark:ring-gray-700 dark:hover:bg-gray-700/80"
            >
              <RefreshCw className="size-3.5" />
              Check now
            </button>
          )}
          {isUpToDate && !hasUpdate && !isChecking && (
            <p className="mt-2 flex items-center justify-center gap-1.5 text-center text-[10px] text-gray-500 dark:text-gray-400">
              <span className="size-1.5 rounded-full bg-emerald-500"></span>
              Up to date
            </p>
          )}
        </div>
      </div>
    </nav>
  );
}
