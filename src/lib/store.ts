import { invoke } from '@tauri-apps/api/core';

export interface AppSettings {
  hotkey: string;
  theme: 'light' | 'dark';
  autostart: boolean;
  recordingMode: 'hold' | 'toggle';
  selectedMic: string;
  selectedModel: string;
}

export const DEFAULTS: AppSettings = {
  hotkey: 'ctrl+win',
  theme: 'light',
  autostart: false,
  recordingMode: 'hold',
  selectedMic: 'System Default',
  selectedModel: '',
};

/**
 * Thin store facade backed by Rust SettingsState (Mutex<serde_json::Value>).
 *
 * Replaces tauri-plugin-store — all persistence goes through a single
 * Mutex in the backend, eliminating the dual-write race condition.
 */
export const store = {
  async get<T>(key: string): Promise<T | null> {
    const value = await invoke<unknown>('get_setting', { key });
    if (value === null || value === undefined) return null;
    return value as T;
  },

  async set(key: string, value: unknown): Promise<void> {
    await invoke('set_setting', { key, value });
  },
};

/** @deprecated Use `store.get` / `store.set` directly */
export async function getStore() {
  return store;
}
