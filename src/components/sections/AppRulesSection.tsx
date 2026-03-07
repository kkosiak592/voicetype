import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { X, Crosshair, Check } from 'lucide-react';
import { store } from '../../lib/store';

interface DetectedApp {
  exe_name: string | null;
  window_title: string | null;
}

type DetectState = 'idle' | 'countdown' | 'success' | 'error';

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
  const [detectState, setDetectState] = useState<DetectState>('idle');
  const [countdown, setCountdown] = useState(3);
  const [detectMessage, setDetectMessage] = useState('');
  const dropdownRef = useRef<HTMLDivElement>(null);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const resetTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    invoke<Record<string, AppRule>>('get_app_rules').then(setRules).catch(() => {});
    store.get<boolean>('all_caps').then(v => setGlobalAllCaps(v ?? false)).catch(() => {});
  }, []);

  // Countdown timer for detect flow
  useEffect(() => {
    if (detectState !== 'countdown') return;
    intervalRef.current = setInterval(() => {
      setCountdown(prev => {
        if (prev <= 1) {
          if (intervalRef.current) clearInterval(intervalRef.current);
          // Detect foreground app
          invoke<DetectedApp>('detect_foreground_app')
            .then(detected => {
              if (!detected.exe_name) {
                setDetectMessage('Could not detect app -- try again');
                setDetectState('error');
              } else if (rules[detected.exe_name]) {
                setDetectMessage(`${detected.exe_name} already added`);
                setDetectState('success');
              } else {
                const exeName = detected.exe_name;
                invoke('set_app_rule', { exeName, rule: { all_caps: null } })
                  .then(() => {
                    setRules(prev => ({ ...prev, [exeName]: { all_caps: null } }));
                    if (detected.window_title) {
                      setWindowTitles(prev => ({ ...prev, [exeName]: detected.window_title! }));
                    }
                    setDetectMessage(`Added ${exeName}`);
                    setDetectState('success');
                  })
                  .catch(() => {
                    setDetectMessage('Could not detect app -- try again');
                    setDetectState('error');
                  });
              }
            })
            .catch(() => {
              setDetectMessage('Could not detect app -- try again');
              setDetectState('error');
            });
          return 0;
        }
        return prev - 1;
      });
    }, 1000);

    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
  }, [detectState, rules]);

  // Reset detect state after success/error message display
  useEffect(() => {
    if (detectState !== 'success' && detectState !== 'error') return;
    resetTimeoutRef.current = setTimeout(() => {
      setDetectState('idle');
      setDetectMessage('');
      setCountdown(3);
    }, 2500);
    return () => {
      if (resetTimeoutRef.current) clearTimeout(resetTimeoutRef.current);
    };
  }, [detectState]);

  function handleDetectClick() {
    setDetectState('countdown');
    setCountdown(3);
  }

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

      <div className="mb-4">
        <button
          onClick={handleDetectClick}
          disabled={detectState !== 'idle'}
          className={`inline-flex items-center gap-2 rounded-xl px-4 py-2 text-sm font-medium transition-colors ${
            detectState === 'success'
              ? 'bg-emerald-600 text-white cursor-default'
              : detectState === 'error'
              ? 'bg-amber-600 text-white cursor-default'
              : detectState === 'countdown'
              ? 'bg-emerald-700 text-white cursor-wait'
              : 'bg-emerald-600 hover:bg-emerald-700 text-white'
          }`}
        >
          {detectState === 'idle' && (
            <>
              <Crosshair className="size-4" />
              Detect Active App
            </>
          )}
          {detectState === 'countdown' && `Detecting in ${countdown}...`}
          {detectState === 'success' && (
            <>
              <Check className="size-4" />
              {detectMessage}
            </>
          )}
          {detectState === 'error' && detectMessage}
        </button>
      </div>

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
