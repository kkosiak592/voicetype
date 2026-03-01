import { Store } from '@tauri-apps/plugin-store';

export interface AppSettings {
  hotkey: string;
  theme: 'light' | 'dark';
  autostart: boolean;
  recordingMode: 'hold' | 'toggle';
}

export const DEFAULTS: AppSettings = {
  hotkey: 'ctrl+shift+space',
  theme: 'light',
  autostart: false,
  recordingMode: 'hold',
};

let _store: Store | null = null;

export async function getStore(): Promise<Store> {
  if (!_store) {
    _store = await Store.load('settings.json', {
      defaults: DEFAULTS as unknown as Record<string, unknown>,
      autoSave: 100,
    });
  }
  return _store;
}
