import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { X } from 'lucide-react';
import { store } from '../../lib/store';

interface AppRule {
  all_caps: boolean | null;
}

type DropdownValue = boolean | null;

function dropdownLabel(value: DropdownValue, globalAllCaps: boolean): string {
  if (value === null) return `Inherit (${globalAllCaps ? 'ON' : 'OFF'})`;
  if (value === true) return 'Force ON';
  return 'Force OFF';
}

export function AppRulesSection() {
  const [rules, setRules] = useState<Record<string, AppRule>>({});
  const [windowTitles, setWindowTitles] = useState<Record<string, string>>({});
  const [globalAllCaps, setGlobalAllCaps] = useState(false);
  const [openDropdown, setOpenDropdown] = useState<string | null>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    invoke<Record<string, AppRule>>('get_app_rules').then(setRules).catch(() => {});
    store.get<boolean>('all_caps').then(v => setGlobalAllCaps(v ?? false)).catch(() => {});
  }, []);

  // Close dropdown on outside click
  useEffect(() => {
    if (!openDropdown) return;
    function handleClick(e: MouseEvent) {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setOpenDropdown(null);
      }
    }
    document.addEventListener('mousedown', handleClick);
    return () => document.removeEventListener('mousedown', handleClick);
  }, [openDropdown]);

  async function handleSetRule(exeName: string, allCaps: DropdownValue) {
    await invoke('set_app_rule', { exeName, rule: { all_caps: allCaps } });
    setRules(prev => ({ ...prev, [exeName]: { all_caps: allCaps } }));
    setOpenDropdown(null);
  }

  async function handleRemoveRule(exeName: string) {
    await invoke('remove_app_rule', { exeName });
    setRules(prev => {
      const next = { ...prev };
      delete next[exeName];
      return next;
    });
    setWindowTitles(prev => {
      const next = { ...prev };
      delete next[exeName];
      return next;
    });
  }

  const sortedEntries = Object.entries(rules).sort(([a], [b]) => a.localeCompare(b));

  const dropdownOptions: { label: string; value: DropdownValue }[] = [
    { label: `Inherit (${globalAllCaps ? 'ON' : 'OFF'})`, value: null },
    { label: 'Force ON', value: true },
    { label: 'Force OFF', value: false },
  ];

  return (
    <div>
      <div className="mb-4">
        <h1 className="text-xl font-bold tracking-tight text-gray-900 dark:text-gray-100">
          App Rules
        </h1>
        <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
          Per-app ALL CAPS overrides. Apps not listed inherit the global default.{' '}
          <span className="font-medium">
            Global default: {globalAllCaps ? 'ON' : 'OFF'}
          </span>
        </p>
      </div>

      {/* Detect button placeholder - implemented in Task 2 */}

      <div className="bg-white dark:bg-gray-900 ring-1 ring-gray-200 dark:ring-gray-800 rounded-2xl p-4 shadow-sm">
        {sortedEntries.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-8">
            <p className="text-sm text-gray-500 dark:text-gray-400">No app rules configured</p>
            <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">
              Use the Detect Active App button to add an application
            </p>
          </div>
        ) : (
          <div>
            {sortedEntries.map(([exeName, rule], idx) => (
              <div
                key={exeName}
                className={`flex items-center justify-between py-3 ${
                  idx < sortedEntries.length - 1
                    ? 'border-b border-gray-100 dark:border-gray-800'
                    : ''
                }`}
              >
                <div className="min-w-0">
                  <p className="font-semibold text-sm text-gray-900 dark:text-gray-100 truncate">
                    {exeName}
                  </p>
                  {windowTitles[exeName] && (
                    <p className="text-xs text-gray-500 dark:text-gray-400 truncate">
                      {windowTitles[exeName]}
                    </p>
                  )}
                </div>

                <div className="flex items-center gap-2 flex-shrink-0">
                  {/* Three-state dropdown */}
                  <div className="relative" ref={openDropdown === exeName ? dropdownRef : undefined}>
                    <button
                      onClick={() =>
                        setOpenDropdown(prev => (prev === exeName ? null : exeName))
                      }
                      className={`inline-flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-medium ring-1 transition-colors ${
                        rule.all_caps === null
                          ? 'text-gray-700 dark:text-gray-300 ring-gray-200 dark:ring-gray-700 bg-gray-50 dark:bg-gray-800'
                          : rule.all_caps === true
                          ? 'text-emerald-700 dark:text-emerald-400 ring-emerald-200 dark:ring-emerald-800 bg-emerald-50 dark:bg-emerald-900/20'
                          : 'text-rose-700 dark:text-rose-400 ring-rose-200 dark:ring-rose-800 bg-rose-50 dark:bg-rose-900/20'
                      }`}
                    >
                      {dropdownLabel(rule.all_caps, globalAllCaps)}
                      <svg
                        className="size-3 opacity-50"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                        strokeWidth={2}
                      >
                        <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
                      </svg>
                    </button>

                    {openDropdown === exeName && (
                      <div className="absolute right-0 z-10 mt-1 w-44 rounded-lg bg-white dark:bg-gray-800 shadow-lg ring-1 ring-gray-200 dark:ring-gray-700 py-1">
                        {dropdownOptions.map(opt => (
                          <button
                            key={String(opt.value)}
                            onClick={() => handleSetRule(exeName, opt.value)}
                            className={`w-full text-left px-3 py-1.5 text-xs font-medium transition-colors hover:bg-gray-100 dark:hover:bg-gray-700 ${
                              opt.value === rule.all_caps
                                ? 'text-emerald-600 dark:text-emerald-400'
                                : 'text-gray-700 dark:text-gray-300'
                            }`}
                          >
                            {opt.label}
                          </button>
                        ))}
                      </div>
                    )}
                  </div>

                  {/* Delete button */}
                  <button
                    onClick={() => handleRemoveRule(exeName)}
                    className="p-1 text-gray-400 hover:text-red-500 transition-colors rounded"
                    title="Remove rule"
                  >
                    <X className="size-4" />
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
